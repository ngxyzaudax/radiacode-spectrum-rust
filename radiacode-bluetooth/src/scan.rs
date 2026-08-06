#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedDevice {
    pub address: String,
    pub local_name: Option<String>,
    pub rssi: Option<i16>,
    pub serial: Option<String>,
    pub model: Option<String>,
}

impl ScannedDevice {
    pub fn display_label(&self) -> String {
        self.model
            .clone()
            .or_else(|| self.serial.clone())
            .unwrap_or_else(|| "RadiaCode".into())
    }
}
