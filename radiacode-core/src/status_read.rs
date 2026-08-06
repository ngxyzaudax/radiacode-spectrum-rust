use crate::command::VirtSfr;
use crate::data_buf::DataBufSnapshot;
use crate::device::RadiaCode;
use crate::error::Result;
use crate::types::DeviceStatus;

pub fn merge_status(carry: &mut DeviceStatus, fresh: DeviceStatus) {
    if let Some(value) = fresh.battery_percent {
        carry.battery_percent = Some(value);
    }
    if let Some(value) = fresh.temperature_c {
        carry.temperature_c = Some(value);
    }
    if let Some(value) = fresh.rssi_dbm {
        carry.rssi_dbm = Some(value);
    }
}

pub async fn status_from_snapshot(
    device: &mut RadiaCode,
    snapshot: &DataBufSnapshot,
    refresh_rssi: bool,
) -> Result<DeviceStatus> {
    let battery_percent = snapshot
        .rare
        .map(|status| status.battery_percent)
        .filter(valid_battery);
    let temperature_c = snapshot
        .rare
        .map(|status| status.temperature_c)
        .filter(valid_temperature)
        .or(read_temperature_c(device).await.ok().filter(valid_temperature));
    let rssi_dbm = if refresh_rssi {
        device.sample_rssi_dbm().await
    } else {
        device.rssi_dbm().await
    };
    Ok(DeviceStatus {
        battery_percent,
        temperature_c,
        rssi_dbm,
    })
}

async fn read_temperature_c(device: &mut RadiaCode) -> Result<f32> {
    device.read_vsfr_f32(VirtSfr::TempDegC).await
}

fn valid_battery(value: &f32) -> bool {
    (0.0..=150.0).contains(value)
}

fn valid_temperature(value: &f32) -> bool {
    (-40.0..=85.0).contains(value)
}
