use crate::monitor::state::{MonitorSample, MonitorState};

const WINDOW_SECS: f64 = 120.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlotBounds {
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
}

#[derive(Clone, Copy)]
pub enum PlotSeries {
    Dose,
    Count,
}

pub fn series_points(monitor: &MonitorState, series: PlotSeries, bounds: PlotBounds) -> Vec<[f64; 2]> {
    monitor
        .history
        .iter()
        .filter(|sample| sample_in_window(sample, bounds))
        .map(|sample| [elapsed_secs(sample), series_value(sample, series)])
        .collect()
}

pub fn plot_bounds(monitor: &MonitorState, series: PlotSeries) -> PlotBounds {
    let x_max = monitor
        .history
        .back()
        .map(elapsed_secs)
        .unwrap_or(0.0)
        .max(1.0);
    let x_min = if x_max > WINDOW_SECS {
        x_max - WINDOW_SECS
    } else {
        0.0
    };
    let window = PlotBounds {
        x_min,
        x_max,
        y_min: 0.0,
        y_max: 1.0,
    };
    let visible: Vec<f64> = monitor
        .history
        .iter()
        .filter(|sample| sample_in_window(sample, window))
        .map(|sample| series_value(sample, series))
        .collect();
    let (mut y_min, mut y_max) = value_range(&visible);
    if let Some(limits) = monitor.limits {
        let (alarm_one, alarm_two) = alarm_values(limits, series);
        y_min = y_min.min(f64::from(alarm_one.min(alarm_two)));
        y_max = y_max.max(f64::from(alarm_one.max(alarm_two)));
    }
    if !visible.is_empty() {
        let span = (y_max - y_min).max(1e-6);
        y_min = (y_min - span * 0.08).max(0.0);
        y_max += span * 0.12;
    }
    if (y_max - y_min).abs() < 1e-6 {
        y_max = y_min + 1.0;
    }
    PlotBounds {
        x_min,
        x_max,
        y_min,
        y_max,
    }
}

fn sample_in_window(sample: &MonitorSample, bounds: PlotBounds) -> bool {
    let seconds = elapsed_secs(sample);
    seconds >= bounds.x_min && seconds <= bounds.x_max
}

fn elapsed_secs(sample: &MonitorSample) -> f64 {
    sample.elapsed.as_secs_f64()
}

fn series_value(sample: &MonitorSample, series: PlotSeries) -> f64 {
    match series {
        PlotSeries::Dose => f64::from(sample.dose_rate.max(0.0)),
        PlotSeries::Count => f64::from(sample.count_rate.max(0.0)),
    }
}

fn alarm_values(limits: radiacode_bluetooth::AlarmLimits, series: PlotSeries) -> (f32, f32) {
    match series {
        PlotSeries::Dose => (
            limits.l1_dose_rate.max(0.0),
            limits.l2_dose_rate.max(0.0),
        ),
        PlotSeries::Count => (
            limits.l1_count_rate.max(0.0),
            limits.l2_count_rate.max(0.0),
        ),
    }
}

fn value_range(values: &[f64]) -> (f64, f64) {
    if values.is_empty() {
        return (0.0, 1.0);
    }
    let mut y_min = f64::INFINITY;
    let mut y_max = 0.0_f64;
    for value in values {
        y_min = y_min.min(*value);
        y_max = y_max.max(*value);
    }
    (y_min, y_max)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{plot_bounds, PlotSeries};
    use crate::monitor::state::{MonitorSample, MonitorState};

    fn sample(seconds: f64, dose: f32, count: f32) -> MonitorSample {
        MonitorSample {
            dose_rate: dose,
            count_rate: count,
            elapsed: Duration::from_secs_f64(seconds),
        }
    }

    #[test]
    fn window_scrolls_with_latest_sample() {
        let mut monitor = MonitorState::default_for_tests();
        monitor.history.push_back(sample(10.0, 1.0, 10.0));
        monitor.history.push_back(sample(150.0, 2.0, 20.0));
        let bounds = plot_bounds(&monitor, PlotSeries::Dose);
        assert!((bounds.x_min - 30.0).abs() < 0.01);
        assert!((bounds.x_max - 150.0).abs() < 0.01);
    }

    #[test]
    fn y_range_follows_visible_data() {
        let mut monitor = MonitorState::default_for_tests();
        monitor.history.push_back(sample(1.0, 10.0, 100.0));
        monitor.history.push_back(sample(2.0, 12.0, 120.0));
        let bounds = plot_bounds(&monitor, PlotSeries::Dose);
        assert!(bounds.y_min < 10.0);
        assert!(bounds.y_max > 12.0);
        assert!(bounds.y_min >= 0.0);
    }
}
