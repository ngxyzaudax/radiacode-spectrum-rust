use radiacode_core::{DeviceEndpoint, DeviceMetadata, DeviceStatus, merge_status, Spectrum, TransportKind};

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub model: String,
    pub serial: String,
    pub firmware: String,
    pub transport: TransportKind,
    pub address: String,
    pub energy_calib: [f32; 3],
    pub battery_percent: Option<f32>,
    pub temperature_c: Option<f32>,
    pub rssi_dbm: Option<i16>,
}

impl DeviceInfo {
    pub fn from_metadata(
        metadata: DeviceMetadata,
        endpoint: &DeviceEndpoint,
        status: DeviceStatus,
    ) -> Self {
        let firmware = metadata.firmware_label();
        Self {
            model: metadata.model,
            serial: metadata.serial,
            firmware,
            transport: endpoint.transport(),
            address: endpoint.address_label().to_string(),
            energy_calib: metadata.energy_calib,
            battery_percent: status.battery_percent,
            temperature_c: status.temperature_c,
            rssi_dbm: status.rssi_dbm,
        }
    }

    pub fn apply_status(&mut self, status: DeviceStatus) {
        let mut merged = DeviceStatus {
            battery_percent: self.battery_percent,
            temperature_c: self.temperature_c,
            rssi_dbm: self.rssi_dbm,
        };
        merge_status(&mut merged, status);
        self.battery_percent = merged.battery_percent;
        self.temperature_c = merged.temperature_c;
        self.rssi_dbm = merged.rssi_dbm;
    }

    pub fn transport_label(&self) -> &'static str {
        match self.transport {
            TransportKind::Bluetooth => "Bluetooth",
            TransportKind::Usb => "USB",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SpectrumView {
    pub duration: std::time::Duration,
    pub a0: f32,
    pub a1: f32,
    pub a2: f32,
    pub counts: Vec<u32>,
    pub total_counts: u64,
}

impl SpectrumView {
    pub fn from_spectrum(spectrum: Spectrum) -> Self {
        let total_counts = spectrum.counts.iter().map(|&count| count as u64).sum();
        Self {
            duration: spectrum.duration,
            a0: spectrum.a0,
            a1: spectrum.a1,
            a2: spectrum.a2,
            counts: spectrum.counts,
            total_counts,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConnectionState {
    #[default]
    Disconnected,
    Connecting,
    Connected,
}
