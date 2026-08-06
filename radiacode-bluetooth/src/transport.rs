use std::time::Duration;

use btleplug::api::{
    Central, Characteristic, Manager as _, Peripheral as _, ScanFilter, WriteType,
};
use btleplug::platform::{Adapter, Manager, Peripheral};
use futures::StreamExt;
use tokio::time::{timeout, Instant};
use tracing::{debug, info, warn};

use crate::buffer::BytesBuffer;
use crate::device_model::{model_from_advertisement, serial_from_advertisement};
use crate::error::{Error, Result};
use crate::protocol::{framed_request_header, response_matches_request, ResponseAssembler};
use crate::scan::ScannedDevice;
use crate::uuids::{self, CHUNK_SIZE, RESPONSE_TIMEOUT_SECS};

const LINK_SETTLE: Duration = Duration::from_millis(800);
const QUIET_GAP: Duration = Duration::from_millis(120);
const MAX_DRAIN: Duration = Duration::from_millis(2500);
const RECONNECT_COOLDOWN: Duration = Duration::from_millis(2500);
const FRESH_SCAN: Duration = Duration::from_secs(4);

pub struct BluetoothTransport {
    peripheral: Peripheral,
    write_char: Characteristic,
    notify_char: Characteristic,
    notifications: futures::stream::BoxStream<'static, btleplug::api::ValueNotification>,
}

impl BluetoothTransport {
    pub async fn connect(mac: &str) -> Result<Self> {
        info!(%mac, "ble transport connect");
        let adapter = default_adapter().await?;
        let peripheral = resolve_peripheral(&adapter, mac).await?;
        Self::connect_peripheral(peripheral).await
    }

    pub async fn connect_fresh(mac: &str) -> Result<Self> {
        info!(%mac, "ble transport fresh connect");
        let adapter = default_adapter().await?;
        disconnect_cached_peripheral(&adapter, mac).await;
        tokio::time::sleep(RECONNECT_COOLDOWN).await;
        let peripheral = find_peripheral(&adapter, mac, FRESH_SCAN).await?;
        Self::connect_peripheral(peripheral).await
    }

    async fn connect_peripheral(peripheral: Peripheral) -> Result<Self> {
        let address = peripheral.address().to_string();
        if peripheral.is_connected().await.unwrap_or(false) {
            debug!(%address, "disconnecting stale peripheral session");
            let _ = peripheral.disconnect().await;
            tokio::time::sleep(RECONNECT_COOLDOWN).await;
        }
        debug!(%address, "connecting peripheral");
        peripheral.connect().await?;
        debug!(%address, "discovering services");
        peripheral.discover_services().await?;

        let write_char = find_characteristic(&peripheral, uuids::WRITE)?;
        let notify_char = find_characteristic(&peripheral, uuids::NOTIFY)?;
        let _ = peripheral.unsubscribe(&notify_char).await;
        peripheral.subscribe(&notify_char).await?;
        let notifications = peripheral.notifications().await?.boxed();
        let mut transport = Self {
            peripheral,
            write_char,
            notify_char,
            notifications,
        };
        transport.settle_link().await;
        info!(%address, "ble transport ready");
        Ok(transport)
    }

    async fn settle_link(&mut self) {
        tokio::time::sleep(LINK_SETTLE).await;
        self.drain_until_quiet().await;
    }

    pub async fn execute(&mut self, request: &[u8]) -> Result<BytesBuffer> {
        let expected = framed_request_header(request)?;
        debug!(request_len = request.len(), "ble execute request");
        self.drain_until_quiet().await;

        for chunk in request.chunks(CHUNK_SIZE) {
            self.peripheral
                .write(&self.write_char, chunk, WriteType::WithoutResponse)
                .await?;
        }

        let mut assembler = ResponseAssembler::default();
        let deadline = Instant::now() + Duration::from_secs(RESPONSE_TIMEOUT_SECS);
        let mut discarded = 0usize;

        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                warn!(request_len = request.len(), discarded, "ble response timed out");
                self.drain_until_quiet().await;
                return Err(Error::Timeout);
            }

            let next = timeout(remaining, self.notifications.next())
                .await
                .map_err(|_| Error::Timeout)?
                .ok_or(Error::ConnectionClosed)?;

            if let Some(payload) = assembler.push(&next.value)? {
                if response_matches_request(&payload, expected) {
                    debug!(response_len = payload.len(), discarded, "ble response complete");
                    return Ok(BytesBuffer::new(payload));
                }
                discarded += 1;
                warn!(response_len = payload.len(), discarded, "discarding unrelated ble frame");
                assembler = ResponseAssembler::default();
            }
        }
    }

    async fn drain_until_quiet(&mut self) {
        let deadline = Instant::now() + MAX_DRAIN;
        let mut last_received = Instant::now();
        let mut drained = 0usize;
        while Instant::now() < deadline {
            let slice = deadline.saturating_duration_since(Instant::now());
            if slice.is_zero() {
                break;
            }
            let wait = slice.min(Duration::from_millis(25));
            match timeout(wait, self.notifications.next()).await {
                Ok(Some(_)) => {
                    drained += 1;
                    last_received = Instant::now();
                }
                Ok(None) => break,
                Err(_) if last_received.elapsed() >= QUIET_GAP => break,
                Err(_) => {}
            }
        }
        if drained > 0 {
            debug!(drained, "drained stale ble notifications");
        }
    }

    pub async fn disconnect(&self) -> Result<()> {
        info!("ble transport disconnect");
        let _ = self.peripheral.unsubscribe(&self.notify_char).await;
        self.peripheral.disconnect().await?;
        Ok(())
    }

    pub async fn rssi_dbm(&self) -> Option<i16> {
        self.peripheral
            .properties()
            .await
            .ok()
            .flatten()
            .and_then(|props| props.rssi)
    }

    pub async fn sample_rssi_dbm(&self) -> Option<i16> {
        let address = self.peripheral.address().to_string();
        if let Some(rssi) = crate::rssi::read_connected_rssi_dbm(&address).await {
            return Some(rssi);
        }
        self.rssi_dbm().await
    }

    pub(crate) async fn drain_link(&mut self) {
        self.drain_until_quiet().await;
    }
}

async fn disconnect_cached_peripheral(adapter: &Adapter, mac: &str) {
    let target = match normalize_mac(mac) {
        Ok(value) => value,
        Err(_) => return,
    };
    for peripheral in adapter.peripherals().await.unwrap_or_default() {
        if peripheral.address().to_string().to_lowercase() != target {
            continue;
        }
        if peripheral.is_connected().await.unwrap_or(false) {
            debug!(%target, "disconnecting cached peripheral before fresh scan");
            let _ = peripheral.disconnect().await;
        }
    }
}

async fn default_adapter() -> Result<Adapter> {
    let manager = Manager::new().await?;
    manager
        .adapters()
        .await?
        .into_iter()
        .next()
        .ok_or(Error::AdapterNotFound)
}

async fn resolve_peripheral(adapter: &Adapter, mac: &str) -> Result<Peripheral> {
    let target = normalize_mac(mac)?;
    for peripheral in adapter.peripherals().await? {
        if peripheral.address().to_string().to_lowercase() == target {
            debug!(%target, "resolved peripheral from known list");
            return Ok(peripheral);
        }
    }
    debug!(%target, "peripheral not cached, scanning");
    find_peripheral(adapter, mac, Duration::from_secs(3)).await
}

async fn find_peripheral(adapter: &Adapter, mac: &str, duration: Duration) -> Result<Peripheral> {
    let target = normalize_mac(mac)?;
    adapter.start_scan(ScanFilter::default()).await?;
    tokio::time::sleep(duration).await;

    let peripherals = adapter.peripherals().await?;
    adapter.stop_scan().await?;

    for peripheral in peripherals {
        let address = peripheral.address().to_string().to_lowercase();
        if address == target {
            return Ok(peripheral);
        }
    }
    Err(Error::DeviceNotFound)
}

fn find_characteristic(peripheral: &Peripheral, uuid: uuid::Uuid) -> Result<Characteristic> {
    peripheral
        .characteristics()
        .into_iter()
        .find(|c| c.uuid == uuid)
        .ok_or(Error::CharacteristicMissing)
}

fn normalize_mac(mac: &str) -> Result<String> {
    let cleaned = mac.trim().to_lowercase().replace('-', ":");
    let parts: Vec<&str> = cleaned.split(':').collect();
    if parts.len() != 6
        || parts
            .iter()
            .any(|p| p.len() != 2 || !p.chars().all(|c| c.is_ascii_hexdigit()))
    {
        return Err(Error::InvalidAddress(mac.to_string()));
    }
    Ok(cleaned)
}

pub async fn scan_radiacode_devices(duration: Duration) -> Result<Vec<ScannedDevice>> {
    info!(?duration, "starting ble scan");
    let adapter = default_adapter().await?;
    adapter.start_scan(ScanFilter::default()).await?;
    tokio::time::sleep(duration).await;
    let peripherals = adapter.peripherals().await?;
    adapter.stop_scan().await?;
    debug!(peripheral_count = peripherals.len(), "scan collected peripherals");

    let mut found = Vec::new();
    for peripheral in peripherals {
        let Some(props) = peripheral.properties().await? else {
            continue;
        };
        let advertises_service = props.services.iter().any(|u| *u == uuids::SERVICE);
        let name_matches = props
            .local_name
            .as_deref()
            .is_some_and(|n| n.to_ascii_lowercase().contains("radiacode"));
        if !advertises_service && !name_matches {
            continue;
        }
        let local_name = props.local_name.clone();
        let serial = local_name
            .as_deref()
            .and_then(serial_from_advertisement);
        let model = local_name.as_deref().and_then(model_from_advertisement);
        debug!(
            address = %peripheral.address(),
            ?local_name,
            rssi = ?props.rssi,
            "matched radiacode advertisement"
        );
        found.push(ScannedDevice {
            address: peripheral.address().to_string(),
            local_name,
            rssi: props.rssi,
            serial,
            model,
        });
    }
    found.sort_by(|left, right| left.address.cmp(&right.address));
    found.dedup_by(|left, right| left.address == right.address);
    info!(count = found.len(), "ble scan complete");
    Ok(found)
}
