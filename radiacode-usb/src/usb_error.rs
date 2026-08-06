use thiserror::Error;

#[derive(Debug, Error)]
pub enum UsbError {
    #[error("usb device not found")]
    DeviceNotFound,
    #[error("usb permission denied")]
    PermissionDenied,
    #[error("usb read returned empty payload")]
    EmptyRead,
    #[error("usb response length mismatch")]
    LengthMismatch,
    #[error("usb transport error: {0}")]
    Transport(String),
    #[error(transparent)]
    Usb(#[from] rusb::Error),
}

pub fn map_usb_error(error: UsbError) -> radiacode_core::Error {
    match error {
        UsbError::DeviceNotFound => radiacode_core::Error::DeviceNotFound,
        UsbError::PermissionDenied => radiacode_core::Error::UsbPermissionDenied,
        UsbError::EmptyRead | UsbError::LengthMismatch => radiacode_core::Error::ConnectionClosed,
        UsbError::Transport(message) => radiacode_core::Error::TransportUnavailable(message),
        UsbError::Usb(error) if error == rusb::Error::Timeout => radiacode_core::Error::Timeout,
        UsbError::Usb(error) if error == rusb::Error::NoDevice => radiacode_core::Error::ConnectionClosed,
        UsbError::Usb(error) if matches!(error, rusb::Error::Access) => {
            radiacode_core::Error::UsbPermissionDenied
        }
        UsbError::Usb(error) => radiacode_core::Error::TransportUnavailable(error.to_string()),
    }
}

pub fn is_connection_lost(error: &radiacode_core::Error) -> bool {
    error.is_connection_lost()
        || matches!(error, radiacode_core::Error::UsbPermissionDenied)
}
