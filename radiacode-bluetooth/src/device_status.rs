use tracing::debug;

use crate::command::VirtString;
use crate::data_buf::latest_snapshot;
use crate::device::RadiaCode;
use crate::error::Result;
use crate::status_read::status_from_snapshot;
use crate::types::DeviceStatus;

pub async fn device_status(device: &mut RadiaCode) -> Result<DeviceStatus> {
    let response = device.read_virt_string(VirtString::DataBuf).await?;
    let snapshot = latest_snapshot(response.data());
    let status = status_from_snapshot(device, &snapshot, true).await?;
    debug!(?status, "device status loaded");
    Ok(status)
}

impl RadiaCode {
    pub async fn device_status(&mut self) -> Result<DeviceStatus> {
        device_status(self).await
    }
}
