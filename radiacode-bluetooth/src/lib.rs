mod alarm_limits;
mod buffer;
mod command;
mod data_buf;
mod device;
mod device_info;
mod device_model;
mod device_settings;
mod device_status;
mod device_time;
mod error;
mod live_rates;
mod metadata;
mod monitor_poll;
mod protocol;
#[cfg(test)]
mod protocol_tests;
mod rate_units;
mod rssi;
mod scan;
mod session_restore;
mod spectrum;
mod status_read;
mod transport;
mod types;
mod uuids;
mod vsfr_batch;

pub use buffer::BytesBuffer;
pub use command::{Command, VirtSfr, VirtString};
pub use data_buf::{
    latest_rare_status, latest_real_time_rates, latest_snapshot, DataBufSnapshot, RareStatus,
    RealTimeRates,
};
pub use device::RadiaCode;
pub use device_model::{model_from_advertisement, model_from_serial, serial_from_advertisement};
pub use error::{Error, Result};
pub use scan::ScannedDevice;
pub use session_restore::SessionRestore;
pub use transport::{scan_radiacode_devices, BluetoothTransport};
pub use status_read::merge_status;
pub use types::{
    channel_to_energy, AlarmLimits, AlarmLimitsUpdate, DeviceMetadata, DeviceStatus,
    DeviceVersions, FirmwareVersion, LiveRates, Spectrum,
};
pub use rate_units::{count_unit_label, dose_unit_label};
