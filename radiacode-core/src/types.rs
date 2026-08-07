use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub struct FirmwareVersion {
    pub major: u16,
    pub minor: u16,
    pub date: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeviceVersions {
    pub boot: FirmwareVersion,
    pub target: FirmwareVersion,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeviceMetadata {
    pub serial: String,
    pub model: String,
    pub versions: DeviceVersions,
    pub energy_calib: [f32; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DeviceStatus {
    pub battery_percent: Option<f32>,
    pub temperature_c: Option<f32>,
    pub rssi_dbm: Option<i16>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AlarmLimits {
    pub l1_count_rate: f32,
    pub l2_count_rate: f32,
    pub l1_dose_rate: f32,
    pub l2_dose_rate: f32,
    pub l1_dose: f32,
    pub l2_dose: f32,
    pub dose_unit_sv: bool,
    pub count_unit_cpm: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AlarmLimitsUpdate {
    pub l1_count_rate: Option<f32>,
    pub l2_count_rate: Option<f32>,
    pub l1_dose_rate: Option<f32>,
    pub l2_dose_rate: Option<f32>,
    pub l1_dose: Option<f32>,
    pub l2_dose: Option<f32>,
    pub dose_unit_sv: Option<bool>,
    pub count_unit_cpm: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LiveRates {
    pub dose_rate: f32,
    pub count_rate: f32,
    pub dose_unit_sv: bool,
    pub count_unit_cpm: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Spectrum {
    pub duration: Duration,
    pub a0: f32,
    pub a1: f32,
    pub a2: f32,
    pub counts: Vec<u32>,
}

pub fn channel_to_energy(channel: u32, a0: f32, a1: f32, a2: f32) -> f32 {
    let x = channel as f32;
    a0 + a1 * x + a2 * x * x
}
