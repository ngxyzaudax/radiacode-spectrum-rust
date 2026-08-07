use tracing::debug;

use crate::command::VirtSfr;
use crate::device::RadiaCode;
use crate::device_time::set_local_time_now;
use crate::error::Result;
use crate::rate_units::{
    decode_count_alarm, decode_dose_accum, decode_dose_alarm, encode_count_alarm, encode_dose_accum,
    encode_dose_alarm,
};
use crate::types::AlarmLimits;

const CTRL_BUTTONS: u32 = 1 << 0;
const CTRL_CLICKS: u32 = 1 << 1;
const CTRL_DR_ALARM1: u32 = 1 << 2;
const CTRL_DR_ALARM2: u32 = 1 << 3;
const CTRL_DR_OOS: u32 = 1 << 4;
const CTRL_DOSE_ALARM1: u32 = 1 << 5;
const CTRL_DOSE_ALARM2: u32 = 1 << 6;
const CTRL_DOSE_OOS: u32 = 1 << 7;
const CTRL_CONNECTION: u32 = 1 << 8;
const CTRL_POWER: u32 = 1 << 9;
const CTRL_CR_ALARM1: u32 = 1 << 10;
const CTRL_CR_ALARM2: u32 = 1 << 11;
const CTRL_CR_OOS: u32 = 1 << 12;
const CTRL_KNOWN_MASK: u32 = 0x1FFF;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlarmSignalMode {
    Once,
    Continuous,
}

impl AlarmSignalMode {
    pub fn from_raw(raw: u32) -> Self {
        if raw == 0 {
            Self::Continuous
        } else {
            Self::Once
        }
    }

    pub fn as_raw(self) -> u32 {
        match self {
            Self::Continuous => 0,
            Self::Once => 1,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Once => "Once",
            Self::Continuous => "Continuous",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayDirection {
    Auto = 0,
    Right = 1,
    Left = 2,
}

impl DisplayDirection {
    pub fn from_raw(raw: u32) -> Self {
        match raw {
            1 => Self::Right,
            2 => Self::Left,
            _ => Self::Auto,
        }
    }

    pub fn as_raw(self) -> u32 {
        self as u32
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::Right => "Right",
            Self::Left => "Left",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BacklightOffTime {
    Sec5,
    Sec10,
    Sec15,
    Sec30,
    Min2,
    Min5,
}

impl BacklightOffTime {
    pub fn from_raw(raw: u32) -> Self {
        match raw {
            1 => Self::Sec10,
            2 => Self::Sec15,
            3 => Self::Sec30,
            4 => Self::Min2,
            5 => Self::Min5,
            _ => Self::Sec5,
        }
    }

    pub fn as_raw(self) -> u32 {
        match self {
            Self::Sec5 => 0,
            Self::Sec10 => 1,
            Self::Sec15 => 2,
            Self::Sec30 => 3,
            Self::Min2 => 4,
            Self::Min5 => 5,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Sec5 => "5 s",
            Self::Sec10 => "10 s",
            Self::Sec15 => "15 s",
            Self::Sec30 => "30 s",
            Self::Min2 => "2 min",
            Self::Min5 => "5 min",
        }
    }

    pub fn all() -> [Self; 6] {
        [
            Self::Sec5,
            Self::Sec10,
            Self::Sec15,
            Self::Sec30,
            Self::Min2,
            Self::Min5,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignalFlags {
    pub buttons: bool,
    pub clicks: bool,
    pub dose_rate_alarm1: bool,
    pub dose_rate_alarm2: bool,
    pub dose_rate_out_of_scale: bool,
    pub dose_alarm1: bool,
    pub dose_alarm2: bool,
    pub dose_out_of_scale: bool,
    pub connection: bool,
    pub power: bool,
    pub count_rate_alarm1: bool,
    pub count_rate_alarm2: bool,
    pub count_rate_out_of_scale: bool,
    pub reserved: u32,
}

impl SignalFlags {
    pub fn from_raw(raw: u32) -> Self {
        Self {
            buttons: raw & CTRL_BUTTONS != 0,
            clicks: raw & CTRL_CLICKS != 0,
            dose_rate_alarm1: raw & CTRL_DR_ALARM1 != 0,
            dose_rate_alarm2: raw & CTRL_DR_ALARM2 != 0,
            dose_rate_out_of_scale: raw & CTRL_DR_OOS != 0,
            dose_alarm1: raw & CTRL_DOSE_ALARM1 != 0,
            dose_alarm2: raw & CTRL_DOSE_ALARM2 != 0,
            dose_out_of_scale: raw & CTRL_DOSE_OOS != 0,
            connection: raw & CTRL_CONNECTION != 0,
            power: raw & CTRL_POWER != 0,
            count_rate_alarm1: raw & CTRL_CR_ALARM1 != 0,
            count_rate_alarm2: raw & CTRL_CR_ALARM2 != 0,
            count_rate_out_of_scale: raw & CTRL_CR_OOS != 0,
            reserved: raw & !CTRL_KNOWN_MASK,
        }
    }

    pub fn as_raw(self) -> u32 {
        let mut raw = self.reserved;
        if self.buttons {
            raw |= CTRL_BUTTONS;
        }
        if self.clicks {
            raw |= CTRL_CLICKS;
        }
        if self.dose_rate_alarm1 {
            raw |= CTRL_DR_ALARM1;
        }
        if self.dose_rate_alarm2 {
            raw |= CTRL_DR_ALARM2;
        }
        if self.dose_rate_out_of_scale {
            raw |= CTRL_DR_OOS;
        }
        if self.dose_alarm1 {
            raw |= CTRL_DOSE_ALARM1;
        }
        if self.dose_alarm2 {
            raw |= CTRL_DOSE_ALARM2;
        }
        if self.dose_out_of_scale {
            raw |= CTRL_DOSE_OOS;
        }
        if self.connection {
            raw |= CTRL_CONNECTION;
        }
        if self.power {
            raw |= CTRL_POWER;
        }
        if self.count_rate_alarm1 {
            raw |= CTRL_CR_ALARM1;
        }
        if self.count_rate_alarm2 {
            raw |= CTRL_CR_ALARM2;
        }
        if self.count_rate_out_of_scale {
            raw |= CTRL_CR_OOS;
        }
        raw
    }

    pub fn without_clicks(self) -> Self {
        Self {
            clicks: false,
            ..self
        }
    }

    pub fn vibro_events(self) -> Self {
        Self {
            clicks: false,
            connection: false,
            power: false,
            ..self
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeviceConfig {
    pub alarms: AlarmLimits,
    pub alarm_mode: AlarmSignalMode,
    pub brightness: u8,
    pub backlight_off: BacklightOffTime,
    pub display_dir: DisplayDirection,
    pub sound_on: bool,
    pub vibro_on: bool,
    pub leds_on: bool,
    pub leds_known: bool,
    pub sound_ctrl: SignalFlags,
    pub vibro_ctrl: SignalFlags,
}

pub async fn load_device_config(device: &mut RadiaCode) -> Result<DeviceConfig> {
    let ids = [
        VirtSfr::CrLev1Cp10s,
        VirtSfr::CrLev2Cp10s,
        VirtSfr::DrLev1UrH,
        VirtSfr::DrLev2UrH,
        VirtSfr::DsLev1Ur,
        VirtSfr::DsLev2Ur,
        VirtSfr::DsUnits,
        VirtSfr::CrUnits,
        VirtSfr::AlarmMode,
        VirtSfr::DispBrt,
        VirtSfr::DispOffTime,
        VirtSfr::DispDir,
        VirtSfr::SoundOn,
        VirtSfr::SoundCtrl,
        VirtSfr::VibroOn,
        VirtSfr::VibroCtrl,
    ];
    let values = device.read_vsfr_batch(&ids).await?;
    let dose_unit_sv = values[6] != 0;
    let count_unit_cpm = values[7] != 0;
    let (leds_on, leds_known) = match device.read_vsfr_u32(VirtSfr::LedsOn).await {
        Ok(value) => (value != 0, true),
        Err(_) => (false, false),
    };
    let config = DeviceConfig {
        alarms: AlarmLimits {
            l1_count_rate: decode_count_alarm(values[0], count_unit_cpm),
            l2_count_rate: decode_count_alarm(values[1], count_unit_cpm),
            l1_dose_rate: decode_dose_alarm(values[2], dose_unit_sv),
            l2_dose_rate: decode_dose_alarm(values[3], dose_unit_sv),
            l1_dose: decode_dose_accum(values[4], dose_unit_sv),
            l2_dose: decode_dose_accum(values[5], dose_unit_sv),
            dose_unit_sv,
            count_unit_cpm,
        },
        alarm_mode: AlarmSignalMode::from_raw(values[8]),
        brightness: values[9].min(9) as u8,
        backlight_off: BacklightOffTime::from_raw(values[10]),
        display_dir: DisplayDirection::from_raw(values[11]),
        sound_on: values[12] != 0,
        sound_ctrl: SignalFlags::from_raw(values[13]),
        vibro_on: values[14] != 0,
        vibro_ctrl: SignalFlags::from_raw(values[15]).vibro_events(),
        leds_on,
        leds_known,
    };
    debug!(?config, "device config loaded");
    Ok(config)
}

pub async fn apply_device_config(device: &mut RadiaCode, config: &DeviceConfig) -> Result<()> {
    let dose_unit_sv = config.alarms.dose_unit_sv;
    let count_unit_cpm = config.alarms.count_unit_cpm;
    let pairs = vec![
        (
            VirtSfr::CrLev1Cp10s,
            encode_count_alarm(config.alarms.l1_count_rate, count_unit_cpm),
        ),
        (
            VirtSfr::CrLev2Cp10s,
            encode_count_alarm(config.alarms.l2_count_rate, count_unit_cpm),
        ),
        (
            VirtSfr::DrLev1UrH,
            encode_dose_alarm(config.alarms.l1_dose_rate, dose_unit_sv),
        ),
        (
            VirtSfr::DrLev2UrH,
            encode_dose_alarm(config.alarms.l2_dose_rate, dose_unit_sv),
        ),
        (
            VirtSfr::DsLev1Ur,
            encode_dose_accum(config.alarms.l1_dose, dose_unit_sv),
        ),
        (
            VirtSfr::DsLev2Ur,
            encode_dose_accum(config.alarms.l2_dose, dose_unit_sv),
        ),
        (VirtSfr::DsUnits, u32::from(dose_unit_sv)),
        (VirtSfr::CrUnits, u32::from(count_unit_cpm)),
        (VirtSfr::AlarmMode, config.alarm_mode.as_raw()),
        (VirtSfr::DispBrt, u32::from(config.brightness.min(9))),
        (VirtSfr::DispOffTime, config.backlight_off.as_raw()),
        (VirtSfr::DispDir, config.display_dir.as_raw()),
        (VirtSfr::SoundOn, u32::from(config.sound_on)),
        (VirtSfr::SoundCtrl, config.sound_ctrl.as_raw()),
        (VirtSfr::VibroOn, u32::from(config.vibro_on)),
        (
            VirtSfr::VibroCtrl,
            config.vibro_ctrl.vibro_events().as_raw(),
        ),
    ];
    device.write_vsfr_batch(&pairs).await?;
    if config.leds_known {
        let leds_on = u32::from(config.leds_on).to_le_bytes();
        if let Err(error) = device.write_vsfr(VirtSfr::LedsOn, &leds_on).await {
            debug!(%error, "LedsOn write skipped");
        }
    }
    debug!(count = pairs.len(), "device config applied");
    Ok(())
}

pub async fn sync_device_clock(device: &mut RadiaCode) -> Result<()> {
    set_local_time_now(device).await?;
    device
        .write_vsfr(VirtSfr::DeviceTime, &0u32.to_le_bytes())
        .await?;
    Ok(())
}

impl RadiaCode {
    pub async fn load_device_config(&mut self) -> Result<DeviceConfig> {
        load_device_config(self).await
    }

    pub async fn apply_device_config(&mut self, config: &DeviceConfig) -> Result<()> {
        apply_device_config(self, config).await
    }

    pub async fn sync_device_clock(&mut self) -> Result<()> {
        sync_device_clock(self).await
    }
}

#[cfg(test)]
mod tests {
    use super::{BacklightOffTime, SignalFlags};

    #[test]
    fn backlight_round_trip() {
        for value in BacklightOffTime::all() {
            assert_eq!(BacklightOffTime::from_raw(value.as_raw()), value);
        }
    }

    #[test]
    fn signal_flags_round_trip_sound_ctrl() {
        let flags = SignalFlags::from_raw(0x1f9d);
        assert!(flags.buttons);
        assert!(!flags.clicks);
        assert!(flags.dose_rate_alarm1);
        assert!(flags.connection);
        assert!(flags.power);
        assert!(flags.count_rate_alarm1);
        assert!(flags.count_rate_alarm2);
        assert!(flags.count_rate_out_of_scale);
        assert_eq!(flags.as_raw(), 0x1f9d);
    }
}
