use tracing::debug;

use crate::command::VirtString;
use crate::data_buf::{latest_snapshot, RealTimeRates};
use crate::device::RadiaCode;
use crate::error::{Error, Result};
use crate::rate_units::{count_display_from_cps, dose_display_from_rh};
use crate::types::{AlarmLimits, LiveRates};

pub async fn live_rates(device: &mut RadiaCode, units: &AlarmLimits) -> Result<LiveRates> {
    let response = device.read_virt_string(VirtString::DataBuf).await?;
    let snapshot = latest_snapshot(response.data());
    let rates = snapshot
        .rates
        .ok_or(Error::MonitorDataPending)
        .map(|raw| to_live_rates(&raw, units))?;
    debug!(?rates, "live rates from databuf");
    Ok(rates)
}

fn to_live_rates(rates: &RealTimeRates, units: &AlarmLimits) -> LiveRates {
    LiveRates {
        dose_rate: dose_display_from_rh(rates.dose_rate_rh, units.dose_unit_sv),
        count_rate: count_display_from_cps(rates.count_rate_cps, units.count_unit_cpm),
        dose_unit_sv: units.dose_unit_sv,
        count_unit_cpm: units.count_unit_cpm,
    }
}

impl RadiaCode {
    pub async fn live_rates(&mut self, units: &AlarmLimits) -> Result<LiveRates> {
        live_rates(self, units).await
    }
}
