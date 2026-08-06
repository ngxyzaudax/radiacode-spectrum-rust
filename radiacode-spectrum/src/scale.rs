#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum YScale {
    #[default]
    Linear,
    Logarithmic,
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

pub fn y_axis_top(peak: f64, scale: YScale) -> f64 {
    if peak <= 0.0 {
        return match scale {
            YScale::Linear => 1.0,
            YScale::Logarithmic => 1.0,
        };
    }
    match scale {
        YScale::Linear => (peak * 1.08).max(1.0),
        YScale::Logarithmic => (peak * 1.08).max(0.5),
    }
}

#[cfg(test)]
mod tests {
    use super::{display_value, y_axis_top, YScale};

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
}
