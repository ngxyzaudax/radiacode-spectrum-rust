use egui::{Grid, RichText, Ui};

use radiacode_bluetooth::{count_unit_label, dose_unit_label};

use crate::monitor::state::MonitorState;
use crate::theme::MUTED;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonitorControlsAction {
    ApplyLimits,
}

pub fn draw_monitor_controls(
    ui: &mut Ui,
    monitor: &mut MonitorState,
    connected: bool,
    busy: bool,
) -> Option<MonitorControlsAction> {
    ui.heading("Alarm thresholds");
    ui.label(
        RichText::new("Values are written to the detector on apply.")
            .small()
            .color(MUTED),
    );
    ui.add_space(10.0);
    let Some(limits) = monitor.limits else {
        ui.label(RichText::new("Connect a device to load alarm limits.").color(MUTED));
        return None;
    };
    let dose_unit = dose_unit_label(limits.dose_unit_sv);
    let count_unit = count_unit_label(limits.count_unit_cpm);
    ui.group(|ui| {
        ui.set_min_width(ui.available_width());
        ui.label(RichText::new("Dose rate").strong());
        ui.add_space(4.0);
        draw_alarm_grid(
            ui,
            &mut monitor.draft.l1_dose_rate,
            &mut monitor.draft.l2_dose_rate,
            dose_unit,
            0.01,
        );
    });
    ui.add_space(10.0);
    ui.group(|ui| {
        ui.set_min_width(ui.available_width());
        ui.label(RichText::new("Count rate").strong());
        ui.add_space(4.0);
        draw_alarm_grid(
            ui,
            &mut monitor.draft.l1_count_rate,
            &mut monitor.draft.l2_count_rate,
            count_unit,
            1.0,
        );
    });
    ui.add_space(12.0);
    let dirty = monitor.limits_dirty();
    let enabled = connected && !busy && dirty;
    if ui
        .add_enabled(enabled, egui::Button::new("Apply to device"))
        .clicked()
    {
        return Some(MonitorControlsAction::ApplyLimits);
    }
    if dirty {
        ui.label(RichText::new("Unsaved changes").small().color(MUTED));
    }
    None
}

fn draw_alarm_grid(
    ui: &mut Ui,
    alarm_one: &mut f32,
    alarm_two: &mut f32,
    unit: &str,
    speed: f64,
) {
    Grid::new(format!("alarm_grid_{unit}"))
        .num_columns(2)
        .spacing([8.0, 6.0])
        .show(ui, |ui| {
            ui.label("Alarm 1 (warning)");
            draw_alarm_value(ui, alarm_one, unit, speed);
            ui.end_row();
            ui.label("Alarm 2 (danger)");
            draw_alarm_value(ui, alarm_two, unit, speed);
            ui.end_row();
        });
}

fn draw_alarm_value(ui: &mut Ui, value: &mut f32, unit: &str, speed: f64) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.add(
            egui::DragValue::new(value)
                .speed(speed)
                .range(0.0..=f64::MAX),
        );
        ui.label(RichText::new(unit).small().color(MUTED));
    });
}
