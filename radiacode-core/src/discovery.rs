#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportKind {
    Bluetooth,
    Usb,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DeviceEndpoint {
    Bluetooth { address: String },
    Usb { serial: String },
}

impl DeviceEndpoint {
    pub fn transport(&self) -> TransportKind {
        match self {
            Self::Bluetooth { .. } => TransportKind::Bluetooth,
            Self::Usb { .. } => TransportKind::Usb,
        }
    }

    pub fn address_label(&self) -> &str {
        match self {
            Self::Bluetooth { address } => address,
            Self::Usb { serial } => serial,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredDevice {
    pub endpoint: DeviceEndpoint,
    pub label: String,
    pub serial: Option<String>,
    pub model: Option<String>,
    pub rssi: Option<i16>,
}

impl DiscoveredDevice {
    pub fn display_label(&self) -> String {
        if !self.label.is_empty() {
            return self.label.clone();
        }
        self.model
            .clone()
            .or_else(|| self.serial.clone())
            .unwrap_or_else(|| "RadiaCode".into())
    }

    pub fn transport_tag(&self) -> &'static str {
        match self.endpoint.transport() {
            TransportKind::Bluetooth => "Bluetooth",
            TransportKind::Usb => "USB",
        }
    }
}

pub fn merge_discovered(
    usb_devices: Vec<DiscoveredDevice>,
    bluetooth_devices: Vec<DiscoveredDevice>,
) -> Vec<DiscoveredDevice> {
    let mut merged = usb_devices;
    for device in bluetooth_devices {
        let duplicate = device
            .serial
            .as_ref()
            .is_some_and(|serial| merged.iter().any(|entry| entry.serial.as_deref() == Some(serial.as_str())));
        if !duplicate {
            merged.push(device);
        }
    }
    merged
}

pub fn resolve_usb_endpoint(
    devices: &[DiscoveredDevice],
    preferred: &DeviceEndpoint,
) -> DeviceEndpoint {
    let usb_devices: Vec<&DiscoveredDevice> = devices
        .iter()
        .filter(|device| device.endpoint.transport() == TransportKind::Usb)
        .collect();
    if usb_devices.is_empty() {
        return preferred.clone();
    }
    if usb_devices.len() == 1 {
        return usb_devices[0].endpoint.clone();
    }
    let preferred_key = preferred.address_label();
    if let Some(device) = usb_devices
        .iter()
        .find(|device| device.endpoint.address_label() == preferred_key)
    {
        return device.endpoint.clone();
    }
    if let Some(device) = usb_devices
        .iter()
        .find(|device| device.serial.as_deref() == Some(preferred_key))
    {
        return device.endpoint.clone();
    }
    usb_devices[0].endpoint.clone()
}
