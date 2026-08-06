use thiserror::Error;

#[derive(Debug, Error)]
pub enum BleError {
    #[error("bluetooth adapter not found")]
    AdapterNotFound,
    #[error("device not found")]
    DeviceNotFound,
    #[error("required BLE characteristic missing")]
    CharacteristicMissing,
    #[error("invalid bluetooth address: {0}")]
    InvalidAddress(String),
    #[error(transparent)]
    Bluetooth(#[from] btleplug::Error),
}

pub fn map_ble_error(error: BleError) -> radiacode_core::Error {
    match error {
        BleError::AdapterNotFound => {
            radiacode_core::Error::TransportUnavailable("bluetooth adapter not found".into())
        }
        BleError::DeviceNotFound => radiacode_core::Error::DeviceNotFound,
        BleError::CharacteristicMissing => {
            radiacode_core::Error::TransportUnavailable("required BLE characteristic missing".into())
        }
        BleError::InvalidAddress(value) => {
            radiacode_core::Error::TransportUnavailable(format!("invalid bluetooth address: {value}"))
        }
        BleError::Bluetooth(error) if is_bluetooth_connection_lost(&error) => {
            radiacode_core::Error::ConnectionClosed
        }
        BleError::Bluetooth(error) => {
            radiacode_core::Error::TransportUnavailable(error.to_string())
        }
    }
}

pub fn is_bluetooth_connection_lost(error: &btleplug::Error) -> bool {
    let message = error.to_string().to_ascii_lowercase();
    message.contains("not connected")
        || message.contains("disconnected")
        || message.contains("device not found")
        || message.contains("link has been lost")
        || message.contains("broken pipe")
        || message.contains("connection reset")
}

pub fn is_connection_lost(error: &radiacode_core::Error) -> bool {
    error.is_connection_lost()
        || matches!(
            error,
            radiacode_core::Error::TransportUnavailable(message)
                if message.to_ascii_lowercase().contains("not connected")
                    || message.to_ascii_lowercase().contains("disconnected")
                    || message.to_ascii_lowercase().contains("link has been lost")
        )
}
