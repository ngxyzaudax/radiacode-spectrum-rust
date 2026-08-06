use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::Duration;

use radiacode_bluetooth::{
    merge_status, scan_radiacode_devices, AlarmLimits, AlarmLimitsUpdate, DeviceStatus, Error,
    RadiaCode, SessionRestore,
};
use tracing::{debug, error, info, warn};

use crate::model::{DeviceInfo, SpectrumView};
use crate::worker::WorkerEvent;

const CONNECT_COOLDOWN: Duration = Duration::from_millis(1500);
const TRANSIENT_RETRIES: usize = 2;
const ALARM_REFRESH_POLLS: u64 = 120;

#[derive(Clone)]
pub struct SessionEpoch {
    pub live: Arc<AtomicU64>,
    pub started: u64,
}

impl SessionEpoch {
    pub fn active(&self) -> bool {
        self.live.load(Ordering::SeqCst) == self.started
    }
}

pub async fn handle_scan(events: &Sender<WorkerEvent>) {
    info!("scanning for radiacode devices");
    match scan_radiacode_devices(Duration::from_secs(5)).await {
        Ok(devices) => {
            info!(count = devices.len(), "scan finished");
            let _ = events.send(WorkerEvent::ScanFinished(devices));
        }
        Err(error) => {
            error!(%error, "scan failed");
            let _ = events.send(WorkerEvent::Error(error.to_string()));
        }
    }
}

pub async fn handle_connect(
    events: &Sender<WorkerEvent>,
    previous: Option<RadiaCode>,
    mac: &str,
    session: &SessionEpoch,
    link_status: &mut DeviceStatus,
    session_restore: &mut Option<SessionRestore>,
) -> Option<RadiaCode> {
    info!(%mac, "connecting");
    if let Some(previous) = previous {
        debug!(%mac, "disconnecting previous session before connect");
        let _ = previous.disconnect().await;
        tokio::time::sleep(CONNECT_COOLDOWN).await;
    }
    if !session.active() {
        warn!(%mac, "connect aborted: session ended");
        return None;
    }
    match RadiaCode::connect(mac).await {
        Ok(mut device) => {
            if !session.active() {
                warn!(%mac, "connect aborted after link up: session ended");
                let _ = device.disconnect().await;
                return None;
            }
            match load_device_info(&mut device, mac, events).await {
                Ok(info) => {
                    if !session.active() {
                        warn!(%mac, "connect aborted after metadata load: session ended");
                        let _ = device.disconnect().await;
                        return None;
                    }
                    *link_status = DeviceStatus {
                        battery_percent: info.battery_percent,
                        temperature_c: info.temperature_c,
                        rssi_dbm: info.rssi_dbm,
                    };
                    *session_restore = device.session_restore();
                    info!(
                        %mac,
                        serial = %info.serial,
                        model = %info.model,
                        "connected"
                    );
                    let _ = events.send(WorkerEvent::Connected(info));
                    Some(device)
                }
                Err(error) => {
                    error!(%mac, %error, "failed to load device info");
                    let _ = device.disconnect().await;
                    let _ = events.send(WorkerEvent::Error(error.to_string()));
                    let _ = events.send(WorkerEvent::Disconnected);
                    None
                }
            }
        }
        Err(error) => {
            error!(%mac, %error, "connect failed");
            let _ = events.send(WorkerEvent::Error(error.to_string()));
            let _ = events.send(WorkerEvent::Disconnected);
            None
        }
    }
}

async fn load_device_info(
    device: &mut RadiaCode,
    mac: &str,
    events: &Sender<WorkerEvent>,
) -> radiacode_bluetooth::Result<DeviceInfo> {
    debug!(%mac, "loading device metadata");
    let metadata = device.metadata().await?;
    let status = device.device_status().await.unwrap_or_default();
    if let Ok(limits) = device.alarm_limits().await {
        let _ = events.send(WorkerEvent::AlarmLimits(limits));
    }
    Ok(DeviceInfo::from_metadata(metadata, mac, status))
}

pub async fn handle_disconnect(events: &Sender<WorkerEvent>, device: Option<RadiaCode>) {
    info!("disconnect requested");
    if let Some(device) = device {
        if let Err(error) = device.disconnect().await {
            error!(%error, "disconnect failed");
            let _ = events.send(WorkerEvent::Error(error.to_string()));
        }
    }
    let _ = events.send(WorkerEvent::Disconnected);
}

pub async fn handle_spectrum(
    events: &Sender<WorkerEvent>,
    device: Option<RadiaCode>,
    session_mac: Option<&str>,
    session: &SessionEpoch,
    link_status: &mut DeviceStatus,
    session_restore: &Option<SessionRestore>,
) -> Option<RadiaCode> {
    let Some(mut device) = device else {
        warn!("spectrum fetch skipped: no active device");
        return None;
    };
    match fetch_spectrum_with_retries(&mut device).await {
        Ok(spectrum) => {
            if !session.active() {
                return Some(device);
            }
            debug!(
                channels = spectrum.counts.len(),
                duration_secs = spectrum.duration.as_secs(),
                "spectrum fetched"
            );
            let _ = events.send(WorkerEvent::Spectrum(SpectrumView::from_spectrum(spectrum)));
            Some(device)
        }
        Err(error) => {
            handle_device_error(
                events,
                device,
                session_mac,
                error,
                session,
                link_status,
                session_restore,
            )
            .await
        }
    }
}

pub async fn handle_reset(
    events: &Sender<WorkerEvent>,
    device: Option<RadiaCode>,
    session_mac: Option<&str>,
    session: &SessionEpoch,
    link_status: &mut DeviceStatus,
    session_restore: &Option<SessionRestore>,
) -> Option<RadiaCode> {
    let Some(mut device) = device else {
        warn!("spectrum reset skipped: no active device");
        return None;
    };
    info!("resetting spectrum");
    match device.spectrum_reset().await {
        Ok(()) => {
            handle_spectrum(
                events,
                Some(device),
                session_mac,
                session,
                link_status,
                session_restore,
            )
            .await
        }
        Err(error) => {
            handle_device_error(
                events,
                device,
                session_mac,
                error,
                session,
                link_status,
                session_restore,
            )
            .await
        }
    }
}

async fn fetch_spectrum_with_retries(
    device: &mut RadiaCode,
) -> radiacode_bluetooth::Result<radiacode_bluetooth::Spectrum> {
    let mut last_error: Option<Error> = None;
    for attempt in 0..=TRANSIENT_RETRIES {
        if attempt > 0 {
            debug!(attempt, "retrying spectrum fetch after transient error");
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        match device.spectrum().await {
            Ok(spectrum) => return Ok(spectrum),
            Err(error) if error.is_transient() => {
                warn!(attempt, %error, "transient spectrum error");
                last_error = Some(error);
            }
            Err(error) => return Err(error),
        }
    }
    Err(last_error.unwrap_or(Error::Timeout))
}

async fn handle_device_error(
    events: &Sender<WorkerEvent>,
    device: RadiaCode,
    session_mac: Option<&str>,
    error: Error,
    session: &SessionEpoch,
    link_status: &mut DeviceStatus,
    session_restore: &Option<SessionRestore>,
) -> Option<RadiaCode> {
    if should_reconnect(&error, session_mac) {
        warn!(%error, session_mac, "connection lost during device operation");
        drop(device);
        if session.active() {
            return reconnect_and_restore(
                events,
                session_mac?,
                session,
                link_status,
                session_restore,
            )
            .await;
        }
        warn!("skipping reconnect: session ended");
        return None;
    }
    error!(%error, "device operation failed");
    let _ = events.send(WorkerEvent::Error(error.to_string()));
    Some(device)
}

fn should_reconnect(error: &Error, session_mac: Option<&str>) -> bool {
    session_mac.is_some() && (error.is_connection_lost() || matches!(error, Error::Timeout))
}

async fn reconnect_and_restore(
    events: &Sender<WorkerEvent>,
    mac: &str,
    session: &SessionEpoch,
    link_status: &mut DeviceStatus,
    session_restore: &Option<SessionRestore>,
) -> Option<RadiaCode> {
    if !session.active() {
        warn!(%mac, "reconnect skipped: session ended");
        return None;
    }
    let Some(restore) = session_restore else {
        warn!(%mac, "reconnect skipped: no cached session");
        let _ = events.send(WorkerEvent::Disconnected);
        return None;
    };
    info!(%mac, "attempting reconnect");
    let _ = events.send(WorkerEvent::Reconnecting);
    match reconnect(mac, session, restore).await {
        Ok(mut device) => {
            if !session.active() {
                warn!(%mac, "reconnect aborted after link up: session ended");
                let _ = device.disconnect().await;
                return None;
            }
            *link_status = DeviceStatus::default();
            match load_device_info(&mut device, mac, events).await {
                Ok(info) => {
                    if !session.active() {
                        let _ = device.disconnect().await;
                        return None;
                    }
                    *link_status = DeviceStatus {
                        battery_percent: info.battery_percent,
                        temperature_c: info.temperature_c,
                        rssi_dbm: info.rssi_dbm,
                    };
                    info!(%mac, serial = %info.serial, "reconnected");
                    let _ = events.send(WorkerEvent::Connected(info));
                    match fetch_spectrum_with_retries(&mut device).await {
                        Ok(spectrum) => {
                            if session.active() {
                                let _ = events.send(WorkerEvent::Spectrum(SpectrumView::from_spectrum(
                                    spectrum,
                                )));
                            }
                            Some(device)
                        }
                        Err(error) => {
                            error!(%mac, %error, "spectrum fetch failed after reconnect");
                            let _ = events.send(WorkerEvent::Error(error.to_string()));
                            Some(device)
                        }
                    }
                }
                Err(error) => {
                    error!(%mac, %error, "reconnect session restore failed");
                    let _ = device.disconnect().await;
                    let _ = events.send(WorkerEvent::Error(error.to_string()));
                    let _ = events.send(WorkerEvent::Disconnected);
                    None
                }
            }
        }
        Err(error) => {
            if session.active() {
                error!(%mac, %error, "reconnect failed");
                let _ = events.send(WorkerEvent::Error(error.to_string()));
                let _ = events.send(WorkerEvent::Disconnected);
            } else {
                warn!(%mac, "reconnect aborted: session ended");
            }
            None
        }
    }
}

pub async fn handle_monitor(
    events: &Sender<WorkerEvent>,
    device: Option<RadiaCode>,
    session_mac: Option<&str>,
    alarm_limits: &mut Option<AlarmLimits>,
    monitor_polls: &mut u64,
    session: &SessionEpoch,
    link_status: &mut DeviceStatus,
    session_restore: &Option<SessionRestore>,
) -> Option<RadiaCode> {
    let Some(mut device) = device else {
        warn!("monitor fetch skipped: no active device");
        return None;
    };
    let limits = match ensure_alarm_limits(&mut device, alarm_limits, events, monitor_polls).await {
        Ok(limits) => limits,
        Err(error) => {
            return handle_device_error(
                events,
                device,
                session_mac,
                error,
                session,
                link_status,
                session_restore,
            )
            .await
        }
    };
    let refresh_rssi = true;
    match device.poll_monitor(&limits, refresh_rssi).await {
        Ok((rates, fresh)) => {
            merge_status(link_status, fresh);
            if !session.active() {
                return Some(device);
            }
            if let Some(rates) = rates {
                *monitor_polls = monitor_polls.saturating_add(1);
                let _ = events.send(WorkerEvent::MonitorSample(rates));
            } else {
                debug!("monitor rates not yet available in databuf");
            }
            let _ = events.send(WorkerEvent::DeviceStatus(*link_status));
            let _ = events.send(WorkerEvent::MonitorPollComplete);
            Some(device)
        }
        Err(error) => {
            handle_device_error(
                events,
                device,
                session_mac,
                error,
                session,
                link_status,
                session_restore,
            )
            .await
        }
    }
}

pub async fn handle_set_alarm_limits(
    events: &Sender<WorkerEvent>,
    device: Option<RadiaCode>,
    session_mac: Option<&str>,
    update: AlarmLimitsUpdate,
    alarm_limits: &mut Option<AlarmLimits>,
    session: &SessionEpoch,
    link_status: &mut DeviceStatus,
    session_restore: &Option<SessionRestore>,
) -> Option<RadiaCode> {
    let Some(mut device) = device else {
        warn!("alarm update skipped: no active device");
        return None;
    };
    match device.set_alarm_limits(&update).await {
        Ok(()) => match device.alarm_limits().await {
            Ok(limits) => {
                if session.active() {
                    *alarm_limits = Some(limits);
                    let _ = events.send(WorkerEvent::AlarmLimits(limits));
                }
                Some(device)
            }
            Err(error) => {
                handle_device_error(
                    events,
                    device,
                    session_mac,
                    error,
                    session,
                    link_status,
                    session_restore,
                )
                .await
            }
        },
        Err(error) => {
            handle_device_error(
                events,
                device,
                session_mac,
                error,
                session,
                link_status,
                session_restore,
            )
            .await
        }
    }
}

async fn ensure_alarm_limits(
    device: &mut RadiaCode,
    cache: &mut Option<AlarmLimits>,
    events: &Sender<WorkerEvent>,
    monitor_polls: &u64,
) -> radiacode_bluetooth::Result<AlarmLimits> {
    let refresh = cache.is_none() || monitor_polls.is_multiple_of(ALARM_REFRESH_POLLS);
    if refresh {
        let limits = device.alarm_limits().await?;
        *cache = Some(limits);
        let _ = events.send(WorkerEvent::AlarmLimits(limits));
    }
    Ok(cache.expect("alarm limits cached"))
}

async fn reconnect(
    mac: &str,
    session: &SessionEpoch,
    restore: &SessionRestore,
) -> radiacode_bluetooth::Result<RadiaCode> {
    if !session.active() {
        return Err(Error::ConnectionClosed);
    }
    RadiaCode::reconnect_session(mac, restore).await
}
