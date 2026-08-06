use crate::device::RadiaCode;
use crate::device_info::{energy_calib, serial_number};
use crate::device_model::model_from_serial;
use crate::error::{Error, Result};
use crate::types::DeviceMetadata;

pub async fn metadata(device: &mut RadiaCode) -> Result<DeviceMetadata> {
    let serial = serial_number(device).await?;
    let energy_calib = energy_calib(device).await?;
    let versions = device
        .cached_versions()
        .ok_or(Error::ProtocolMismatch {
            expected: "initialized device".into(),
            got: "missing cached firmware version".into(),
        })?
        .clone();
    Ok(DeviceMetadata {
        serial: serial.clone(),
        model: model_from_serial(&serial),
        versions,
        energy_calib,
    })
}

impl DeviceMetadata {
    pub fn firmware_label(&self) -> String {
        format!(
            "{}.{} ({})",
            self.versions.target.major, self.versions.target.minor, self.versions.target.date
        )
    }
}

impl RadiaCode {
    pub async fn metadata(&mut self) -> Result<DeviceMetadata> {
        metadata(self).await
    }
}
