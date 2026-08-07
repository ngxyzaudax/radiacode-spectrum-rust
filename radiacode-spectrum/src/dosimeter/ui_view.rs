use egui::{RichText, Ui, Vec2b};
use egui_plot::{HoverPosition, Line, Plot, PlotPoints, Points};

use radiacode_core::dose_accum_unit_label;

use crate::dosimeter::plot_bounds::{dose_points, plot_bounds, PlotBounds};
use crate::dosimeter::state::{format_session_duration, DosimeterState};
use crate::monitor::AlarmLevel;
use crate::theme::{ACCENT, MUTED};

pub fn draw_dosimeter_view(ui: &mut Ui, dosimeter: &DosimeterState) {
    let Some(latest) = dosimeter.latest else {
        ui.add_space(12.0);
        ui.label(RichText::new(&dosimeter.status).color(MUTED));
        return;
    };
    let unit = dose_accum_unit_label(latest.dose_unit_sv);
    let level = dosimeter.dose_alarm_level();
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new("Accumulated dose").small().color(MUTED));
            ui.label(
                RichText::new(format!("{:.2}", latest.dose.max(0.0)))
                    .size(36.0)
                    .color(alarm_color(level)),
            );
            ui.label(RichText::new(unit).small().color(MUTED));
        });
        ui.add_space(24.0);
        ui.vertical(|ui| {
            ui.label(RichText::new("Session").small().color(MUTED));
            ui.label(
                RichText::new(format_session_duration(latest.duration_secs))
                    .size(24.0),
            );
        });
    });
    ui.add_space(16.0);
    draw_dose_plot(ui, dosimeter, unit);
}

fn draw_dose_plot(ui: &mut Ui, dosimeter: &DosimeterState, unit: &str) {
    ui.label(RichText::new("Cumulative dose").strong());
    ui.label(
        RichText::new("Session accumulation since last reset")
            .small()
            .color(MUTED),
    );
    let bounds = plot_bounds(dosimeter);
    let points = dose_points(dosimeter, bounds);
    let unit_label = unit.to_string();
    Plot::new("dosimeter_dose_plot")
        .height(280.0)
        .allow_zoom(false)
        .allow_drag(false)
        .allow_scroll(false)
        .auto_bounds(Vec2b::new(false, false))
        .x_axis_label("Session time (s)")
        .y_axis_label(unit)
        .label_formatter(move |pos| match pos {
            HoverPosition::NearDataPoint { position, .. } => Some(format!(
                "Time: {:.0} s\nDose: {:.2} {unit_label}",
                position.x, position.y
            )),
            _ => None,
        })
        .show(ui, |plot_ui| {
            plot_ui.set_plot_bounds_x(bounds.x_min..=bounds.x_max);
            plot_ui.set_plot_bounds_y(bounds.y_min..=bounds.y_max);
            if points.len() >= 2 {
                plot_ui.line(
                    Line::new("dose", PlotPoints::from(points.clone())).color(ACCENT),
                );
            } else if points.len() == 1 {
                plot_ui.points(
                    Points::new("dose", PlotPoints::from(points))
                        .radius(4.0)
                        .color(ACCENT),
                );
            }
            if let Some(limits) = dosimeter.limits {
                draw_alarm_lines(
                    plot_ui,
                    bounds,
                    limits.l1_dose.max(0.0),
                    limits.l2_dose.max(0.0),
                );
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
