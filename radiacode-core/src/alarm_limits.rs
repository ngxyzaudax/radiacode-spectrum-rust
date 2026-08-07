use tracing::debug;

use crate::command::VirtSfr;
use crate::device::RadiaCode;
use crate::error::Result;
use crate::rate_units::{
    decode_count_alarm, decode_dose_accum, decode_dose_alarm, encode_count_alarm, encode_dose_accum,
    encode_dose_alarm,
};
use crate::types::{AlarmLimits, AlarmLimitsUpdate};

pub async fn alarm_limits(device: &mut RadiaCode) -> Result<AlarmLimits> {
    let ids = [
        VirtSfr::CrLev1Cp10s,
        VirtSfr::CrLev2Cp10s,
        VirtSfr::DrLev1UrH,
        VirtSfr::DrLev2UrH,
        VirtSfr::DsLev1Ur,
        VirtSfr::DsLev2Ur,
        VirtSfr::DsUnits,
        VirtSfr::CrUnits,
    ];
    let values = device.read_vsfr_batch(&ids).await?;
    let dose_unit_sv = values[6] != 0;
    let count_unit_cpm = values[7] != 0;
    let limits = AlarmLimits {
        l1_count_rate: decode_count_alarm(values[0], count_unit_cpm),
        l2_count_rate: decode_count_alarm(values[1], count_unit_cpm),
        l1_dose_rate: decode_dose_alarm(values[2], dose_unit_sv),
        l2_dose_rate: decode_dose_alarm(values[3], dose_unit_sv),
        l1_dose: decode_dose_accum(values[4], dose_unit_sv),
        l2_dose: decode_dose_accum(values[5], dose_unit_sv),
        dose_unit_sv,
        count_unit_cpm,
    };
    debug!(?limits, "alarm limits loaded");
    Ok(limits)
}

pub async fn set_alarm_limits(device: &mut RadiaCode, update: &AlarmLimitsUpdate) -> Result<()> {
    let current = alarm_limits(device).await?;
    let dose_unit_sv = update.dose_unit_sv.unwrap_or(current.dose_unit_sv);
    let count_unit_cpm = update.count_unit_cpm.unwrap_or(current.count_unit_cpm);
    let mut pairs = Vec::new();
    if let Some(value) = update.l1_count_rate {
        pairs.push((VirtSfr::CrLev1Cp10s, encode_count_alarm(value, count_unit_cpm)));
    }
    if let Some(value) = update.l2_count_rate {
        pairs.push((VirtSfr::CrLev2Cp10s, encode_count_alarm(value, count_unit_cpm)));
    }
    if let Some(value) = update.l1_dose_rate {
        pairs.push((VirtSfr::DrLev1UrH, encode_dose_alarm(value, dose_unit_sv)));
    }
    if let Some(value) = update.l2_dose_rate {
        pairs.push((VirtSfr::DrLev2UrH, encode_dose_alarm(value, dose_unit_sv)));
    }
    if let Some(value) = update.l1_dose {
        pairs.push((VirtSfr::DsLev1Ur, encode_dose_accum(value, dose_unit_sv)));
    }
    if let Some(value) = update.l2_dose {
        pairs.push((VirtSfr::DsLev2Ur, encode_dose_accum(value, dose_unit_sv)));
    }
    if let Some(value) = update.dose_unit_sv {
        pairs.push((VirtSfr::DsUnits, u32::from(value)));
    }
    if let Some(value) = update.count_unit_cpm {
        pairs.push((VirtSfr::CrUnits, u32::from(value)));
    }
    if pairs.is_empty() {
        return Ok(());
    }
    device.write_vsfr_batch(&pairs).await?;
    debug!(count = pairs.len(), "alarm limits written");
    Ok(())
}

impl RadiaCode {
    pub async fn alarm_limits(&mut self) -> Result<AlarmLimits> {
        alarm_limits(self).await
    }

    pub async fn set_alarm_limits(&mut self, update: &AlarmLimitsUpdate) -> Result<()> {
        set_alarm_limits(self, update).await
    }
}
