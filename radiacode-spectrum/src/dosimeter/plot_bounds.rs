use crate::dosimeter::state::DosimeterState;

const Y_HEADROOM: f64 = 0.2;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlotBounds {
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
}

pub fn dose_points(dosimeter: &DosimeterState, bounds: PlotBounds) -> Vec<[f64; 2]> {
    dosimeter
        .history
        .iter()
        .filter(|point| point.duration_secs as f64 >= bounds.x_min)
        .map(|point| [point.duration_secs as f64, f64::from(point.dose)])
        .collect()
}

pub fn plot_bounds(dosimeter: &DosimeterState) -> PlotBounds {
    let x_max = dosimeter
        .latest
        .map(|sample| f64::from(sample.duration_secs))
        .unwrap_or(60.0)
        .max(60.0);
    let peak = dosimeter
        .history
        .iter()
        .map(|point| f64::from(point.dose))
        .fold(0.0_f64, f64::max);
    let alarm_peak = dosimeter.limits.map(|limits| {
        f64::from(limits.l1_dose.max(limits.l2_dose).max(0.0))
    });
    let y_max = upper_y(peak, alarm_peak);
    PlotBounds {
        x_min: 0.0,
        x_max,
        y_min: 0.0,
        y_max,
    }
}

fn upper_y(peak: f64, alarm_peak: Option<f64>) -> f64 {
    let base = peak.max(alarm_peak.unwrap_or(0.0)).max(0.001);
    base * (1.0 + Y_HEADROOM)
}
