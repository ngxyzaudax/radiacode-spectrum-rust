use tracing::debug;

use crate::command::VirtString;
use crate::data_buf::latest_snapshot;
use crate::device::RadiaCode;
use crate::error::Result;
use crate::status_read::status_from_snapshot;
use crate::types::DeviceStatus;

pub async fn device_status(device: &mut RadiaCode, refresh_rssi: bool) -> Result<DeviceStatus> {
    let response = device.read_virt_string(VirtString::DataBuf).await?;
    let snapshot = latest_snapshot(response.data());
    let status = status_from_snapshot(device, &snapshot, refresh_rssi).await?;
    debug!(?status, "device status loaded");
    Ok(status)
}

impl RadiaCode {
    pub async fn device_status(&mut self, refresh_rssi: bool) -> Result<DeviceStatus> {
        device_status(self, refresh_rssi).await
    }
}
