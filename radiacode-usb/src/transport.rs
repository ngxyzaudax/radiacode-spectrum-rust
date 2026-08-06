use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use radiacode_core::{
    framed_request_header, response_matches_request, BytesBuffer, DeviceEndpoint, DiscoveredDevice,
    Error, RadiaCode, SessionRestore, Transport, model_from_serial,
};
use rusb::{Context, DeviceHandle, UsbContext};
use tracing::{debug, info, warn};

use crate::constants::{DRAIN, EMPTY_READ_RETRIES, EP_IN, EP_OUT, INTERFACE, PID, READ_BUF, TIMEOUT, VID};
use crate::usb_error::{map_usb_error, UsbError};

type LockedHandle = Arc<Mutex<DeviceHandle<Context>>>;

pub struct UsbTransport {
    _context: Context,
    handle: LockedHandle,
}

impl UsbTransport {
    pub fn connect(serial: &str) -> std::result::Result<Self, UsbError> {
        info!(%serial, "usb transport connect");
        let context = Context::new()?;
        let handle = open_handle(&context, Some(serial))?;
        Ok(Self {
            _context: context,
            handle: Arc::new(Mutex::new(handle)),
        })
    }

    pub fn reconnect(serial: &str) -> std::result::Result<Self, UsbError> {
        Self::connect(serial)
    }

    fn execute_sync(
        handle: &mut DeviceHandle<Context>,
        request: &[u8],
    ) -> std::result::Result<BytesBuffer, UsbError> {
        let expected = framed_request_header(request).map_err(|error| UsbError::Transport(error.to_string()))?;
        debug!(request_len = request.len(), "usb execute request");
        drain_handle(handle)?;
        let written = handle.write_bulk(EP_OUT, request, TIMEOUT)?;
        if written != request.len() {
            warn!(written, expected = request.len(), "partial usb write");
        }
        let payload = read_response_sync(handle)?;
        if !response_matches_request(payload.data(), expected) {
            return Err(UsbError::LengthMismatch);
        }
        Ok(payload)
    }
}

#[async_trait(?Send)]
impl Transport for UsbTransport {
    async fn execute(&mut self, request: &[u8]) -> radiacode_core::Result<BytesBuffer> {
        let handle = Arc::clone(&self.handle);
        let request = request.to_vec();
        tokio::task::spawn_blocking(move || {
            let mut handle = handle.lock().expect("usb handle lock");
            UsbTransport::execute_sync(&mut handle, &request).map_err(map_usb_error)
        })
        .await
        .map_err(|error| Error::TransportUnavailable(error.to_string()))?
    }

    async fn drain_link(&mut self) {
        let handle = Arc::clone(&self.handle);
        let _ = tokio::task::spawn_blocking(move || {
            let mut handle = handle.lock().expect("usb handle lock");
            drain_handle(&mut handle)
        })
        .await;
    }

    async fn disconnect(self: Box<Self>) -> radiacode_core::Result<()> {
        info!("usb transport disconnect");
        Ok(())
    }

    async fn link_rssi_dbm(&self) -> Option<i16> {
        None
    }

    async fn sample_link_rssi_dbm(&self) -> Option<i16> {
        None
    }
}

pub fn scan_usb_devices() -> std::result::Result<Vec<DiscoveredDevice>, UsbError> {
    info!("starting usb scan");
    let context = Context::new()?;
    collect_devices(&context)
}

pub async fn connect(serial: &str) -> radiacode_core::Result<RadiaCode> {
    let transport = tokio::task::spawn_blocking({
        let serial = serial.to_string();
        move || UsbTransport::connect(&serial).map_err(map_usb_error)
    })
    .await
    .map_err(|error| Error::TransportUnavailable(error.to_string()))??;
    RadiaCode::open(Box::new(transport), false, None).await
}

pub async fn reconnect_session(serial: &str, restore: &SessionRestore) -> radiacode_core::Result<RadiaCode> {
    info!(%serial, "radiacode usb reconnect with cached session");
    let transport = tokio::task::spawn_blocking({
        let serial = serial.to_string();
        move || UsbTransport::reconnect(&serial).map_err(map_usb_error)
    })
    .await
    .map_err(|error| Error::TransportUnavailable(error.to_string()))??;
    RadiaCode::open(Box::new(transport), false, Some(restore)).await
}

fn collect_devices(context: &Context) -> std::result::Result<Vec<DiscoveredDevice>, UsbError> {
    let mut found = Vec::new();
    for device in context.devices()?.iter() {
        let descriptor = match device.device_descriptor() {
            Ok(value) => value,
            Err(_) => continue,
        };
        if descriptor.vendor_id() != VID || descriptor.product_id() != PID {
            continue;
        }
        let handle = device.open().ok();
        let serial = read_string(handle.as_ref(), &device, descriptor.serial_number_string_index());
        let product = read_string(handle.as_ref(), &device, descriptor.product_string_index());
        let serial = serial.filter(|value| !value.is_empty());
        let product = product.filter(|value| !value.is_empty());
        let model = serial.as_deref().map(model_from_serial);
        let label = model
            .clone()
            .or_else(|| serial.clone())
            .or(product.clone())
            .unwrap_or_else(|| "RadiaCode".into());
        let endpoint_serial = serial.clone().unwrap_or_else(|| format!("usb-{}", device.address()));
        debug!(?serial, ?product, address = device.address(), "matched usb radiacode");
        found.push(DiscoveredDevice {
            endpoint: DeviceEndpoint::Usb {
                serial: endpoint_serial,
            },
            label,
            serial,
            model,
            rssi: None,
        });
    }
    found.sort_by(|left, right| left.endpoint.address_label().cmp(right.endpoint.address_label()));
    found.dedup_by(|left, right| left.endpoint == right.endpoint);
    info!(count = found.len(), "usb scan complete");
    Ok(found)
}

pub(crate) fn usb_permission_denied() -> bool {
    let Ok(context) = Context::new() else {
        return false;
    };
    matches!(open_handle(&context, None), Err(UsbError::PermissionDenied))
}

fn open_handle(context: &Context, serial: Option<&str>) -> std::result::Result<DeviceHandle<Context>, UsbError> {
    let candidates: Vec<_> = context
        .devices()?
        .iter()
        .filter(|device| {
            device
                .device_descriptor()
                .ok()
                .is_some_and(|descriptor| descriptor.vendor_id() == VID && descriptor.product_id() == PID)
        })
        .collect();
    if candidates.is_empty() {
        return Err(UsbError::DeviceNotFound);
    }
    let device = pick_device(&candidates, serial)?;
    let mut handle = match device.open() {
        Ok(value) => value,
        Err(rusb::Error::Access) => return Err(UsbError::PermissionDenied),
        Err(error) => return Err(error.into()),
    };
    if handle.kernel_driver_active(INTERFACE).unwrap_or(false) {
        handle.detach_kernel_driver(INTERFACE).ok();
    }
    handle.claim_interface(INTERFACE)?;
    drain_handle(&mut handle)?;
    Ok(handle)
}

fn pick_device<'a>(
    devices: &'a [rusb::Device<Context>],
    serial: Option<&str>,
) -> std::result::Result<&'a rusb::Device<Context>, UsbError> {
    let Some(target) = serial else {
        return Ok(&devices[0]);
    };
    if let Some(address) = target.strip_prefix("usb-") {
        if let Some(device) = devices
            .iter()
            .find(|device| device.address().to_string() == address)
        {
            return Ok(device);
        }
    } else if let Some(device) = devices.iter().find(|device| {
        device
            .device_descriptor()
            .ok()
            .and_then(|descriptor| read_string(None, device, descriptor.serial_number_string_index()))
            .is_some_and(|value| value == target)
    }) {
        return Ok(device);
    }
    if devices.len() == 1 {
        return Ok(&devices[0]);
    }
    Err(UsbError::DeviceNotFound)
}

fn read_response_sync(handle: &mut DeviceHandle<Context>) -> std::result::Result<BytesBuffer, UsbError> {
    let mut buffer = vec![0u8; READ_BUF];
    let mut payload = Vec::new();
    let mut expected_len: Option<usize> = None;
    let mut empty_reads = 0usize;
    loop {
        match handle.read_bulk(EP_IN, &mut buffer, TIMEOUT) {
            Ok(0) => {
                empty_reads += 1;
                if empty_reads >= EMPTY_READ_RETRIES {
                    return Err(UsbError::EmptyRead);
                }
            }
            Ok(read) => {
                empty_reads = 0;
                payload.extend_from_slice(&buffer[..read]);
                if expected_len.is_none() && payload.len() >= 4 {
                    let length = u32::from_le_bytes(payload[..4].try_into().unwrap()) as usize;
                    expected_len = Some(length);
                    payload.drain(..4);
                }
                if let Some(length) = expected_len {
                    if payload.len() >= length {
                        payload.truncate(length);
                        return Ok(BytesBuffer::new(payload));
                    }
                }
            }
            Err(rusb::Error::Timeout) if expected_len.is_none() => return Err(UsbError::EmptyRead),
            Err(error) => return Err(error.into()),
        }
    }
}

fn drain_handle(handle: &mut DeviceHandle<Context>) -> std::result::Result<(), UsbError> {
    let mut buffer = vec![0u8; READ_BUF];
    loop {
        match handle.read_bulk(EP_IN, &mut buffer, DRAIN) {
            Ok(0) | Err(rusb::Error::Timeout) => return Ok(()),
            Err(error) => return Err(error.into()),
            Ok(_) => {}
        }
    }
}

fn read_string<T: UsbContext>(
    handle: Option<&DeviceHandle<T>>,
    device: &rusb::Device<T>,
    index: Option<u8>,
) -> Option<String> {
    let index = index?;
    if index == 0 {
        return None;
    }
    let read = |opened: &DeviceHandle<T>| {
        let languages = opened.read_languages(Duration::from_millis(100)).ok()?;
        let language = *languages.first()?;
        opened
            .read_string_descriptor(language, index, Duration::from_millis(100))
            .ok()
    };
    if let Some(handle) = handle {
        return read(handle);
    }
    device.open().ok().and_then(|opened| read(&opened))
}
