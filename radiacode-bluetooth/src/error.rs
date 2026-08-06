use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("bluetooth adapter not found")]
    AdapterNotFound,
    #[error("device not found")]
    DeviceNotFound,
    #[error("required BLE characteristic missing")]
    CharacteristicMissing,
    #[error("response timed out")]
    Timeout,
    #[error("connection closed")]
    ConnectionClosed,
    #[error("protocol mismatch: expected {expected}, got {got}")]
    ProtocolMismatch { expected: String, got: String },
    #[error("unexpected device return code {0}")]
    UnexpectedReturnCode(u32),
    #[error("incompatible firmware {major}.{minor}, >=4.8 required")]
    IncompatibleFirmware { major: u16, minor: u16 },
    #[error("invalid bluetooth address: {0}")]
    InvalidAddress(String),
    #[error("buffer underrun: need {need} bytes, have {have}")]
    BufferUnderrun { need: usize, have: usize },
    #[error("vsfr batch returned no readable registers")]
    VsfrBatchEmpty,
    #[error("live rates not yet available in device buffer")]
    MonitorDataPending,
    #[error(transparent)]
    Bluetooth(#[from] btleplug::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn is_transient(&self) -> bool {
        matches!(
            self,
            Self::Timeout | Self::ProtocolMismatch { .. } | Self::BufferUnderrun { .. }
        )
    }

    pub fn is_connection_lost(&self) -> bool {
        match self {
            Self::ConnectionClosed => true,
            Self::Bluetooth(error) => is_bluetooth_connection_lost(error),
            _ => false,
        }
    }
}

fn is_bluetooth_connection_lost(error: &btleplug::Error) -> bool {
    let message = error.to_string().to_ascii_lowercase();
    message.contains("not connected")
        || message.contains("disconnected")
        || message.contains("device not found")
        || message.contains("link has been lost")
        || message.contains("broken pipe")
        || message.contains("connection reset")
}

#[cfg(test)]
mod tests {
    use super::Error;

    #[test]
    fn timeout_is_transient_not_connection_lost() {
        assert!(Error::Timeout.is_transient());
        assert!(!Error::Timeout.is_connection_lost());
    }

    #[test]
    fn connection_closed_is_link_loss() {
        assert!(Error::ConnectionClosed.is_connection_lost());
        assert!(!Error::ConnectionClosed.is_transient());
    }

    #[test]
    fn protocol_mismatch_is_transient_not_link_loss() {
        let error = Error::ProtocolMismatch {
            expected: "aa".into(),
            got: "bb".into(),
        };
        assert!(error.is_transient());
        assert!(!error.is_connection_lost());
    }
}
