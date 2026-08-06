use tracing::debug;

use crate::command::VirtString;
use crate::data_buf::latest_snapshot;
use crate::device::RadiaCode;
use crate::error::Result;
use crate::rate_units::{count_display_from_cps, dose_display_from_rh};
use crate::status_read::status_from_snapshot;
use crate::types::{AlarmLimits, DeviceStatus, LiveRates};

pub async fn poll_monitor(
    device: &mut RadiaCode,
    units: &AlarmLimits,
    refresh_rssi: bool,
) -> Result<(Option<LiveRates>, DeviceStatus)> {
    let response = device.read_virt_string(VirtString::DataBuf).await?;
    let snapshot = latest_snapshot(response.data());
    let status = status_from_snapshot(device, &snapshot, refresh_rssi).await?;
    let rates = snapshot.rates.map(|raw| LiveRates {
        dose_rate: dose_display_from_rh(raw.dose_rate_rh, units.dose_unit_sv),
        count_rate: count_display_from_cps(raw.count_rate_cps, units.count_unit_cpm),
        dose_unit_sv: units.dose_unit_sv,
        count_unit_cpm: units.count_unit_cpm,
    });
    debug!(?rates, ?status, "monitor poll");
    Ok((rates, status))
}

impl RadiaCode {
    pub async fn poll_monitor(
        &mut self,
        units: &AlarmLimits,
        refresh_rssi: bool,
    ) -> Result<(Option<LiveRates>, DeviceStatus)> {
        poll_monitor(self, units, refresh_rssi).await
    }
}
