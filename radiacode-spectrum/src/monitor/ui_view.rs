use egui::{RichText, Ui, Vec2b};
use egui_plot::{HoverPosition, Line, Plot, PlotPoints, Points};

use radiacode_core::{count_unit_label, dose_unit_label};

use crate::monitor::plot_bounds::{plot_bounds, series_points, PlotBounds, PlotSeries};
use crate::monitor::state::{AlarmLevel, MonitorState};
use crate::theme::{ACCENT, MUTED};

pub fn draw_monitor_view(ui: &mut Ui, monitor: &MonitorState) {
    let Some(latest) = monitor.latest else {
        ui.add_space(12.0);
        ui.label(RichText::new(&monitor.status).color(MUTED));
        return;
    };
    let dose_unit = dose_unit_label(latest.dose_unit_sv);
    let count_unit = count_unit_label(latest.count_unit_cpm);
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width() / 2.0 - 8.0, 72.0),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                draw_readout(
                    ui,
                    "Dose rate",
                    latest.dose_rate,
                    dose_unit,
                    monitor.dose_alarm_level(),
                );
            },
        );
        ui.separator();
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), 72.0),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                draw_readout(
                    ui,
                    "Count rate",
                    latest.count_rate,
                    count_unit,
                    monitor.count_alarm_level(),
                );
            },
        );
    });
    ui.add_space(16.0);
    draw_rate_plot(ui, "monitor_dose_plot", "Dose rate", monitor, PlotSeries::Dose, dose_unit);
    ui.add_space(12.0);
    draw_rate_plot(
        ui,
        "monitor_count_plot",
        "Count rate",
        monitor,
        PlotSeries::Count,
        count_unit,
    );
}

fn draw_readout(
    ui: &mut Ui,
    title: &str,
    value: f32,
    unit: &str,
    level: AlarmLevel,
) {
    let value = value.max(0.0);
    ui.label(RichText::new(title).small().color(MUTED));
    ui.label(
        RichText::new(format!("{value:.2}"))
            .size(32.0)
            .color(alarm_color(level)),
    );
    ui.label(RichText::new(unit).small().color(MUTED));
}

fn draw_rate_plot(
    ui: &mut Ui,
    id: &str,
    title: &str,
    monitor: &MonitorState,
    series: PlotSeries,
    unit: &str,
) {
    ui.label(RichText::new(title).strong());
    let bounds = plot_bounds(monitor, series);
    let window_secs = (bounds.x_max - bounds.x_min).max(0.0);
    ui.label(
        RichText::new(format!("Last {window_secs:.0}s window"))
            .small()
            .color(MUTED),
    );
    let points = series_points(monitor, series, bounds);
    let unit_label = unit.to_string();
    let series_title = title.to_string();
    let hover_title = series_title.clone();
    let hover_unit = unit_label.clone();
    Plot::new(id)
        .height(200.0)
        .allow_zoom(false)
        .allow_drag(false)
        .allow_scroll(false)
        .auto_bounds(Vec2b::new(false, false))
        .x_axis_label("Time (s)")
        .y_axis_label(unit)
        .label_formatter(move |pos| match pos {
            HoverPosition::NearDataPoint {
                plot_name,
                position,
                ..
            } if *plot_name == hover_title => Some(format!(
                "Time: {:.1} s\n{hover_title}: {:.2} {hover_unit}",
                position.x, position.y
            )),
            _ => None,
        })
        .show(ui, |plot_ui| {
            plot_ui.set_plot_bounds_x(bounds.x_min..=bounds.x_max);
            plot_ui.set_plot_bounds_y(bounds.y_min..=bounds.y_max);
            if points.len() >= 2 {
                plot_ui.line(
                    Line::new(&series_title, PlotPoints::from(points.clone())).color(ACCENT),
                );
            } else if points.len() == 1 {
                plot_ui.points(
                    Points::new(&series_title, PlotPoints::from(points))
                        .radius(4.0)
                        .color(ACCENT),
                );
            }
            if let Some(limits) = monitor.limits {
                let (alarm_one, alarm_two) = match series {
                    PlotSeries::Dose => (
                        limits.l1_dose_rate.max(0.0),
                        limits.l2_dose_rate.max(0.0),
                    ),
                    PlotSeries::Count => (
                        limits.l1_count_rate.max(0.0),
                        limits.l2_count_rate.max(0.0),
                    ),
                };
                draw_alarm_lines(plot_ui, bounds, alarm_one, alarm_two);
            }
        });
}

fn draw_alarm_lines(
    plot_ui: &mut egui_plot::PlotUi,
    bounds: PlotBounds,
    alarm_one: f32,
    alarm_two: f32,
) {
    plot_ui.line(
        Line::new(
            "alarm_warning",
            PlotPoints::new(vec![
                [bounds.x_min, f64::from(alarm_one)],
                [bounds.x_max, f64::from(alarm_one)],
            ]),
        )
        .color(egui::Color32::from_rgb(240, 180, 64))
        .allow_hover(false),
    );
    plot_ui.line(
        Line::new(
            "alarm_danger",
            PlotPoints::new(vec![
                [bounds.x_min, f64::from(alarm_two)],
                [bounds.x_max, f64::from(alarm_two)],
            ]),
        )
        .color(egui::Color32::from_rgb(220, 80, 80))
        .allow_hover(false),
    );
}

fn alarm_color(level: AlarmLevel) -> egui::Color32 {
    match level {
        AlarmLevel::Normal => egui::Color32::from_rgb(230, 234, 240),
        AlarmLevel::Warning => egui::Color32::from_rgb(240, 180, 64),
        AlarmLevel::Danger => egui::Color32::from_rgb(220, 80, 80),
    }
}
