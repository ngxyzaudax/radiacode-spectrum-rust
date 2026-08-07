use std::time::{Duration, Instant};

use radiacode_core::{DeviceEndpoint, DiscoveredDevice};
use tracing::{debug, info, warn};

use crate::model::{ConnectionState, DeviceInfo, SpectrumView};
use crate::dosimeter::DosimeterState;
use crate::monitor::MonitorState;
use crate::worker::{WorkerCommand, WorkerEvent};

pub struct AppState {
    pub devices: Vec<DiscoveredDevice>,
    pub connecting_endpoint: Option<DeviceEndpoint>,
    pub connection: ConnectionState,
    pub device_info: Option<DeviceInfo>,
    pub spectrum: Option<SpectrumView>,
    pub spectrum_sequence: u64,
    pub monitor: MonitorState,
    pub dosimeter: DosimeterState,
    pub scanning: bool,
    pub busy: bool,
    pub status: String,
    pub last_fetch: Option<Instant>,
    pub last_monitor_fetch: Option<Instant>,
    pub spectrum_fetch_pending: bool,
    pub monitor_fetch_pending: bool,
    pub scanned_once: bool,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
            connecting_endpoint: None,
            connection: ConnectionState::Disconnected,
            device_info: None,
            spectrum: None,
            spectrum_sequence: 0,
            monitor: MonitorState::new(),
            dosimeter: DosimeterState::new(),
            scanning: false,
            busy: false,
            status: "Looking for nearby detectors…".into(),
            last_fetch: None,
            last_monitor_fetch: None,
            spectrum_fetch_pending: false,
            monitor_fetch_pending: false,
            scanned_once: false,
        }
    }

    pub fn clear_session(&mut self) {
        self.connection = ConnectionState::Disconnected;
        self.connecting_endpoint = None;
        self.device_info = None;
        self.spectrum = None;
        self.spectrum_sequence = 0;
        self.monitor.on_disconnect();
        self.dosimeter.on_disconnect();
        self.last_fetch = None;
        self.last_monitor_fetch = None;
        self.spectrum_fetch_pending = false;
        self.monitor_fetch_pending = false;
        self.status = if self.scanned_once {
            "Disconnected. Choose a device to reconnect.".into()
        } else {
            "Disconnected.".into()
        };
    }

    pub fn apply_event(&mut self, event: WorkerEvent, accept_session: bool) -> Option<WorkerCommand> {
        match event {
            WorkerEvent::Busy(busy) => {
                debug!(busy, "worker busy state");
                self.busy = busy;
                None
            }
            WorkerEvent::ScanFinished(devices) => {
                info!(count = devices.len(), "ui received scan results");
                self.scanning = false;
                self.scanned_once = true;
                self.devices = devices;
                self.status = if self.devices.is_empty() {
                    "No RadiaCode devices found.".into()
                } else {
                    format!("{} device(s) available.", self.devices.len())
                };
                None
            }
            WorkerEvent::Connected(info) if accept_session => {
                info!(
                    serial = %info.serial,
                    address = %info.address,
                    transport = ?info.transport,
                    "ui connected"
                );
                let fresh_session = self.connection != ConnectionState::Connected;
                self.connection = ConnectionState::Connected;
                self.connecting_endpoint = None;
                self.device_info = Some(info);
                if fresh_session {
                    self.monitor.on_connect();
                    self.dosimeter.on_connect();
                }
                self.spectrum_fetch_pending = false;
                self.monitor_fetch_pending = false;
                self.status = "Connected. Acquiring monitor data…".into();
                Some(WorkerCommand::FetchMonitor)
            }
            WorkerEvent::Connected(_) => None,
            WorkerEvent::Disconnected => {
                info!("ui disconnected");
                self.clear_session();
                None
            }
            WorkerEvent::Reconnecting if accept_session => {
                warn!("ui reconnecting after connection loss");
                self.status = "Connection lost, reconnecting…".into();
                self.spectrum = None;
                self.last_fetch = None;
                self.spectrum_fetch_pending = false;
                self.monitor_fetch_pending = false;
                None
            }
            WorkerEvent::Reconnecting => None,
            WorkerEvent::UsbPermissionRequired { endpoint } if accept_session => {
                warn!(?endpoint, "ui usb permission required");
                self.scanning = false;
                self.spectrum_fetch_pending = false;
                self.monitor_fetch_pending = false;
                self.connection = ConnectionState::Disconnected;
                self.connecting_endpoint = Some(endpoint.clone());
                self.status = "USB access required.".into();
                None
            }
            WorkerEvent::UsbPermissionRequired { .. } => None,
            WorkerEvent::Spectrum(spectrum) if accept_session => {
                self.spectrum = Some(spectrum);
                self.spectrum_sequence = self.spectrum_sequence.saturating_add(1);
                self.last_fetch = Some(Instant::now());
                self.spectrum_fetch_pending = false;
                self.status = "Live spectrum".into();
                debug!(sequence = self.spectrum_sequence, "ui spectrum updated");
                None
            }
            WorkerEvent::Spectrum(_) => None,
            WorkerEvent::DeviceStatus(status) if accept_session => {
                self.monitor_fetch_pending = false;
                if let Some(info) = self.device_info.as_mut() {
                    info.apply_status(status);
                    debug!(
                        battery = ?info.battery_percent,
                        temperature = ?info.temperature_c,
                        rssi = ?info.rssi_dbm,
                        "ui device status updated"
                    );
                }
                None
            }
            WorkerEvent::DeviceStatus(_) => None,
            WorkerEvent::MonitorSample(sample) if accept_session => {
                if let Some(rates) = sample.rates {
                    self.monitor.push_sample(rates);
                }
                if let Some(accumulated) = sample.accumulated {
                    self.dosimeter.push_sample(accumulated);
                }
                self.last_monitor_fetch = Some(Instant::now());
                self.monitor_fetch_pending = false;
                self.status = "Live monitor".into();
                None
            }
            WorkerEvent::MonitorSample(_) => None,
            WorkerEvent::DoseResetComplete if accept_session => {
                self.dosimeter.on_reset();
                self.status = "Dose reset.".into();
                Some(WorkerCommand::FetchMonitor)
            }
            WorkerEvent::DoseResetComplete => None,
            WorkerEvent::MonitorPollComplete if accept_session => {
                self.last_monitor_fetch = Some(Instant::now());
                self.monitor_fetch_pending = false;
                None
            }
            WorkerEvent::MonitorPollComplete => None,
            WorkerEvent::AlarmLimits(limits) if accept_session => {
                self.monitor.apply_limits(limits);
                self.dosimeter.apply_limits(limits);
                None
            }
            WorkerEvent::AlarmLimits(_) => None,
            WorkerEvent::DeviceConfig(_) => None,
            WorkerEvent::Error(message) => {
                warn!(%message, "ui received worker error");
                self.scanning = false;
                self.spectrum_fetch_pending = false;
                self.monitor_fetch_pending = false;
                if self.connection == ConnectionState::Connecting {
                    self.connection = ConnectionState::Disconnected;
                    self.connecting_endpoint = None;
                }
                self.status = message;
                None
            }
        }
    }

    pub fn live_refresh_due(&self, enabled: bool, interval_secs: u64) -> bool {
        let connected = self.connection == ConnectionState::Connected;
        let due = self
            .last_fetch
            .map(|t| t.elapsed() >= Duration::from_secs(interval_secs.max(1)))
            .unwrap_or(true);
        enabled && connected && due && !self.spectrum_fetch_pending
    }

    pub fn try_schedule_spectrum(&mut self) -> bool {
        if self.spectrum_fetch_pending {
            return false;
        }
        self.spectrum_fetch_pending = true;
        true
    }

    pub fn try_schedule_monitor(&mut self) -> bool {
        if self.monitor_fetch_pending {
            return false;
        }
        self.monitor_fetch_pending = true;
        true
    }
}
