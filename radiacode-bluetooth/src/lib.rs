mod ble_error;
mod device_model;
mod rssi;
mod transport;
mod uuids;

pub use ble_error::{is_connection_lost, map_ble_error, BleError};
pub use radiacode_core::{
    merge_discovered, AlarmLimits, AlarmLimitsUpdate, DeviceEndpoint, DeviceMetadata,
    DeviceStatus, DiscoveredDevice, Error, LiveRates, RadiaCode, Result, SessionRestore,
    Spectrum, Transport, TransportKind,
};
pub use transport::{connect, reconnect_session, scan_radiacode_devices, BluetoothTransport};
pub use rssi::read_connected_rssi_dbm;
