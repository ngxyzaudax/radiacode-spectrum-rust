use egui::{RichText, Ui, Vec2};

use crate::settings::ui_icons::{paint_signal_icon, SignalIconKind};
use crate::theme::MUTED;

const CARD_WIDTH: f32 = 300.0;
const LABEL_WIDTH: f32 = 56.0;
const VALUE_WIDTH: f32 = 96.0;
const UNIT_WIDTH: f32 = 52.0;
const CHECK_WIDTH: f32 = 22.0;

pub fn alarm_card(
    ui: &mut Ui,
    title: &str,
    warning: &mut f32,
    danger: &mut f32,
    unit: &str,
    speed: f64,
    signals: [(&mut bool, &mut bool); 3],
) {
    let [(sw, vw), (sd, vd), (so, vo)] = signals;
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_min_width(CARD_WIDTH);
            ui.set_max_width(CARD_WIDTH);
            ui.vertical(|ui| {
                card_title(ui, title);
                ui.add_space(6.0);
                threshold_row(ui, "Warn", warning, unit, speed, sw, vw);
                ui.add_space(4.0);
                threshold_row(ui, "Danger", danger, unit, speed, sd, vd);
                ui.add_space(4.0);
                oos_row(ui, so, vo);
            });
        });
}

fn card_title(ui: &mut Ui, title: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(title).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            fixed_check_cell(ui, |ui| {
                paint_signal_icon(ui, SignalIconKind::Vibro, true);
            });
            fixed_check_cell(ui, |ui| {
                paint_signal_icon(ui, SignalIconKind::Sound, true);
            });
        });
    });
}

fn threshold_row(
    ui: &mut Ui,
    label: &str,
    value: &mut f32,
    unit: &str,
    speed: f64,
    sound: &mut bool,
    vibro: &mut bool,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        fixed_label(ui, label);
        fixed_value(ui, value, speed);
        fixed_unit(ui, unit);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            signal_check(ui, vibro, "Vibration");
            signal_check(ui, sound, "Sound");
        });
    });
}

fn oos_row(ui: &mut Ui, sound: &mut bool, vibro: &mut bool) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        fixed_label(ui, "OOS");
        ui.allocate_ui_with_layout(
            Vec2::new(VALUE_WIDTH + UNIT_WIDTH + 6.0, ui.spacing().interact_size.y),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.label(RichText::new("Out of scale").small().color(MUTED));
            },
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            signal_check(ui, vibro, "Vibration");
            signal_check(ui, sound, "Sound");
        });
    });
}

fn fixed_label(ui: &mut Ui, label: &str) {
    ui.allocate_ui_with_layout(
        Vec2::new(LABEL_WIDTH, ui.spacing().interact_size.y),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.set_min_width(LABEL_WIDTH);
            ui.set_max_width(LABEL_WIDTH);
            ui.label(RichText::new(label).small().color(MUTED));
        },
    );
}

fn fixed_value(ui: &mut Ui, value: &mut f32, speed: f64) {
    let height = ui.spacing().interact_size.y;
    ui.add_sized(
        Vec2::new(VALUE_WIDTH, height),
        egui::DragValue::new(value)
            .speed(speed)
            .range(0.0..=f64::MAX)
            .min_decimals(0)
            .max_decimals(2),
    );
}

fn fixed_unit(ui: &mut Ui, unit: &str) {
    ui.allocate_ui_with_layout(
        Vec2::new(UNIT_WIDTH, ui.spacing().interact_size.y),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.set_min_width(UNIT_WIDTH);
            ui.set_max_width(UNIT_WIDTH);
            ui.label(RichText::new(unit).small().color(MUTED));
        },
    );
}

fn signal_check(ui: &mut Ui, checked: &mut bool, tip: &str) {
    fixed_check_cell(ui, |ui| {
        ui.checkbox(checked, "").on_hover_text(tip);
    });
}

fn fixed_check_cell(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    ui.allocate_ui_with_layout(
        Vec2::new(CHECK_WIDTH, ui.spacing().interact_size.y),
        egui::Layout::top_down(egui::Align::Center),
        |ui| {
            ui.set_min_width(CHECK_WIDTH);
            ui.set_max_width(CHECK_WIDTH);
            add(ui);
        },
    );
}
