use egui::{Color32, Ui, Vec2b};
use egui_plot::{Bar, Line, Plot, PlotPoints};

use crate::analysis::ui_plot_geometry::outline_points;
use crate::energy::{bar_energy_width, clamp_energy_range, ENERGY_MAX_KEV, ENERGY_MIN_KEV};
use crate::scale::{display_rate, rate_log_floor, y_axis_top, HistogramStyle, YScale};

pub struct PlotSeries<'a> {
    pub id: &'a str,
    pub bars: &'a [Bar],
    pub color: Color32,
}

pub fn owned_series(
    id: &str,
    energies: &[f64],
    fallback_width: f64,
    values: &[f64],
    color: Color32,
    y_scale: YScale,
    log_floor: f64,
) -> (String, Vec<Bar>, Color32) {
    let bars = energies
        .iter()
        .enumerate()
        .zip(values.iter())
        .map(|((index, &energy), &value)| {
            Bar::new(energy, display_rate(value, y_scale, log_floor))
                .width(bar_energy_width(energies, index, fallback_width))
                .fill(color)
        })
        .collect();
    (id.to_string(), bars, color)
}

pub fn shared_log_floor(value_sets: &[&[f64]]) -> f64 {
    let mut rates = Vec::new();
    for values in value_sets {
        rates.extend_from_slice(values);
    }
    rate_log_floor(&rates)
}

pub fn show_owned_series(
    ui: &mut Ui,
    id: &str,
    owned: &[(String, Vec<Bar>, Color32)],
    y_scale: YScale,
    style: HistogramStyle,
) {
    let series: Vec<PlotSeries<'_>> = owned
        .iter()
        .map(|(series_id, bars, color)| PlotSeries {
            id: series_id.as_str(),
            bars,
            color: *color,
        })
        .collect();
    let peak = series_peak(&series);
    let y_top = y_axis_top(peak, y_scale);
    plot_series(ui, id, &series, y_scale, y_top, style);
}

fn series_peak(series: &[PlotSeries<'_>]) -> f64 {
    series
        .iter()
        .flat_map(|item| item.bars.iter())
        .map(|bar| bar.value)
        .filter(|value| value.is_finite())
        .fold(0.0_f64, f64::max)
}

fn plot_series(
    ui: &mut Ui,
    id: &str,
    series: &[PlotSeries<'_>],
    y_scale: YScale,
    y_top: f64,
    style: HistogramStyle,
) {
    let plot_id = format!("{id}_{y_scale:?}");
    Plot::new(plot_id)
        .allow_zoom(true)
        .allow_drag(true)
        .allow_scroll(true)
        .auto_bounds(Vec2b::new(false, false))
        .default_x_bounds(ENERGY_MIN_KEV, ENERGY_MAX_KEV)
        .include_y(0.0)
        .x_axis_label("Energy (keV)")
        .y_axis_label(y_axis_label(y_scale))
        .show(ui, |plot_ui| {
            let bounds = plot_ui.plot_bounds();
            let (min_x, max_x) = clamp_energy_range(bounds.min()[0], bounds.max()[0]);
            plot_ui.set_plot_bounds_x(min_x..=max_x);
            plot_ui.set_plot_bounds_y(0.0..=y_top);
            for item in series {
                if item.bars.is_empty() {
                    continue;
                }
                let points = outline_points(item.bars);
                let mut line = Line::new(item.id, PlotPoints::from(points))
                    .color(item.color)
                    .width(1.75);
                if style == HistogramStyle::Filled {
                    line = line.fill(0.0).fill_alpha(0.35);
                }
                plot_ui.line(line);
            }
        });
}

fn y_axis_label(scale: YScale) -> &'static str {
    match scale {
        YScale::Linear => "cps",
        YScale::Logarithmic => "cps (log₁₀)",
    }
}
