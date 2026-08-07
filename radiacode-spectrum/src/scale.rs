#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum YScale {
    #[default]
    Linear,
    Logarithmic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HistogramStyle {
    #[default]
    Filled,
    Outline,
}

pub fn display_value(count: f64, scale: YScale) -> f64 {
    match scale {
        YScale::Linear => count.max(0.0),
        YScale::Logarithmic => {
            if count <= 0.0 {
                0.0
            } else {
                count.max(1.0).log10()
            }
        }
    }
}

pub fn display_rate(rate: f64, scale: YScale, log_floor: f64) -> f64 {
    match scale {
        YScale::Linear => rate.max(0.0),
        YScale::Logarithmic => {
            let floor = log_floor.max(1e-12);
            (rate.max(floor) / floor).log10()
        }
    }
}

pub fn rate_log_floor(rates: &[f64]) -> f64 {
    let min_positive = rates
        .iter()
        .copied()
        .filter(|rate| *rate > 0.0)
        .fold(f64::INFINITY, f64::min);
    if min_positive.is_finite() {
        (min_positive * 0.1).clamp(1e-12, 1e-3)
    } else {
        1e-6
    }
}

pub fn y_axis_top(peak: f64, scale: YScale) -> f64 {
    if peak <= 0.0 {
        return 1.0;
    }
    match scale {
        YScale::Linear => peak * 1.08,
        YScale::Logarithmic => (peak * 1.08).max(0.5),
    }
}

#[cfg(test)]
mod tests {
    use super::{display_rate, display_value, rate_log_floor, y_axis_top, YScale};

    #[test]
    fn log_display_never_negative() {
        assert_eq!(display_value(0.4, YScale::Logarithmic), 0.0);
        assert!(display_value(10.0, YScale::Logarithmic) > 0.0);
    }

    #[test]
    fn y_axis_top_tracks_peak() {
        assert!(y_axis_top(100.0, YScale::Linear) < 200.0);
        assert!(y_axis_top(3.0, YScale::Logarithmic) < 5.0);
    }

    #[test]
    fn linear_y_axis_follows_small_peaks() {
        let top = y_axis_top(0.3, YScale::Linear);
        assert!(top > 0.3);
        assert!(top < 0.4);
    }

    #[test]
    fn rate_log_stays_non_negative_for_fractional_cps() {
        let floor = rate_log_floor(&[0.3, 0.1, 0.0]);
        let peak = display_rate(0.3, YScale::Logarithmic, floor);
        let low = display_rate(0.1, YScale::Logarithmic, floor);
        let zero = display_rate(0.0, YScale::Logarithmic, floor);
        assert!(peak > low);
        assert!(low >= 0.0);
        assert_eq!(zero, 0.0);
        assert!(y_axis_top(peak, YScale::Logarithmic) > peak);
    }
}
