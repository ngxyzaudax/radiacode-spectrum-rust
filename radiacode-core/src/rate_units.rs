pub fn dose_display_from_rh(dose_rate_rh: f32, dose_unit_sv: bool) -> f32 {
    if dose_unit_sv {
        dose_rate_rh * 10_000.0
    } else {
        dose_rate_rh * 1_000_000.0
    }
}

pub fn dose_display_from_ur_h(dose_ur_h: f32, dose_unit_sv: bool) -> f32 {
    if dose_unit_sv {
        dose_ur_h / 100.0
    } else {
        dose_ur_h
    }
}

pub fn count_display_from_cps(count_cps: f32, count_unit_cpm: bool) -> f32 {
    if count_unit_cpm {
        count_cps * 60.0
    } else {
        count_cps
    }
}

pub fn encode_dose_alarm(display: f32, dose_unit_sv: bool) -> u32 {
    let multiplier = if dose_unit_sv { 100.0 } else { 1.0 };
    (display * multiplier).round().max(0.0) as u32
}

pub fn decode_dose_alarm(raw_ur_h: u32, dose_unit_sv: bool) -> f32 {
    let divisor = if dose_unit_sv { 100.0 } else { 1.0 };
    raw_ur_h as f32 / divisor
}

pub fn encode_dose_accum(display_micro: f32, dose_unit_sv: bool) -> u32 {
    let multiplier = if dose_unit_sv { 100.0 } else { 1.0 };
    (display_micro * multiplier).round().max(0.0) as u32
}

pub fn decode_dose_accum(raw: u32, dose_unit_sv: bool) -> f32 {
    let divisor = if dose_unit_sv { 100.0 } else { 1.0 };
    raw as f32 / divisor
}

pub fn encode_count_alarm(display: f32, count_unit_cpm: bool) -> u32 {
    let multiplier = if count_unit_cpm { 1.0 / 6.0 } else { 10.0 };
    (display * multiplier).round().max(0.0) as u32
}

pub fn decode_count_alarm(raw_cp10s: u32, count_unit_cpm: bool) -> f32 {
    let multiplier = if count_unit_cpm { 60.0 } else { 1.0 };
    raw_cp10s as f32 / 10.0 * multiplier
}

pub fn dose_unit_label(dose_unit_sv: bool) -> &'static str {
    if dose_unit_sv {
        "µSv/h"
    } else {
        "µR/h"
    }
}

pub fn count_unit_label(count_unit_cpm: bool) -> &'static str {
    if count_unit_cpm {
        "cpm"
    } else {
        "cps"
    }
}

#[cfg(test)]
mod tests {
    use super::{
        count_display_from_cps, decode_count_alarm, decode_dose_alarm, dose_display_from_rh,
        dose_display_from_ur_h, encode_count_alarm, encode_dose_alarm,
    };

    #[test]
    fn dose_sv_round_trip() {
        let display = 1.25;
        let raw = encode_dose_alarm(display, true);
        assert_eq!(raw, 125);
        assert!((decode_dose_alarm(raw, true) - display).abs() < 0.01);
    }

    #[test]
    fn dose_r_round_trip() {
        let display = 125.0;
        let raw = encode_dose_alarm(display, false);
        assert_eq!(raw, 125);
        assert!((decode_dose_alarm(raw, false) - display).abs() < 0.01);
    }

    #[test]
    fn count_cps_round_trip() {
        let display = 42.0;
        let raw = encode_count_alarm(display, false);
        assert_eq!(raw, 420);
        assert!((decode_count_alarm(raw, false) - display).abs() < 0.01);
    }

    #[test]
    fn count_cpm_round_trip() {
        let display = 600.0;
        let raw = encode_count_alarm(display, true);
        assert_eq!(raw, 100);
        assert!((decode_count_alarm(raw, true) - display).abs() < 0.01);
    }

    #[test]
    fn realtime_dose_conversions() {
        let rh = 0.000_125;
        assert!((dose_display_from_rh(rh, true) - 1.25).abs() < 0.001);
        assert!((dose_display_from_rh(rh, false) - 125.0).abs() < 0.1);
    }

    #[test]
    fn vsfr_dose_conversions() {
        assert!((dose_display_from_ur_h(125.0, true) - 1.25).abs() < 0.001);
        assert!((dose_display_from_ur_h(125.0, false) - 125.0).abs() < 0.001);
    }

    #[test]
    fn count_display() {
        assert!((count_display_from_cps(10.0, false) - 10.0).abs() < 0.001);
        assert!((count_display_from_cps(10.0, true) - 600.0).abs() < 0.001);
    }
}
