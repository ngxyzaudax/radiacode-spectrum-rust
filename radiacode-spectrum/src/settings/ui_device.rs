use egui::{RichText, Ui};

use radiacode_core::{BacklightOffTime, DeviceConfig, DisplayDirection};

use crate::model::DeviceInfo;
use crate::theme::MUTED;

pub use crate::settings::ui_alarms::draw_alarms_panel;
pub use crate::settings::ui_signals::draw_signals_panel;

pub fn draw_device_info(ui: &mut Ui, info: Option<&DeviceInfo>) {
    let Some(info) = info else {
        ui.label(RichText::new("Not connected").color(MUTED));
        return;
    };
    ui.horizontal_wrapped(|ui| {
        meta_chip(ui, "Model", &info.model);
        meta_chip(ui, "Serial", &info.serial);
        meta_chip(ui, "Firmware", &info.firmware);
        meta_chip(ui, "Link", info.transport_label());
        if let Some(battery) = info.battery_percent {
            meta_chip(ui, "Battery", &format!("{battery:.0}%"));
        }
        if let Some(temp) = info.temperature_c {
            meta_chip(ui, "Temp", &format!("{temp:.1} °C"));
        }
        if let Some(rssi) = info.rssi_dbm {
            meta_chip(ui, "RSSI", &format!("{rssi} dBm"));
        }
    });
}

pub fn draw_units_panel(ui: &mut Ui, draft: &mut DeviceConfig) {
    ui.horizontal(|ui| {
        ui.label("Dose rate");
        ui.selectable_value(&mut draft.alarms.dose_unit_sv, true, "µSv/h");
        ui.selectable_value(&mut draft.alarms.dose_unit_sv, false, "µR/h");
        ui.add_space(16.0);
        ui.label("Count rate");
        ui.selectable_value(&mut draft.alarms.count_unit_cpm, false, "cps");
        ui.selectable_value(&mut draft.alarms.count_unit_cpm, true, "cpm");
    });
}

pub fn draw_screen_panel(ui: &mut Ui, draft: &mut DeviceConfig) {
    ui.add(egui::Slider::new(&mut draft.brightness, 0..=9).text("Brightness"));
    ui.horizontal(|ui| {
        ui.label("Backlight off");
        egui::ComboBox::from_id_salt("settings_backlight_off")
            .selected_text(draft.backlight_off.label())
            .show_ui(ui, |ui| {
                for value in BacklightOffTime::all() {
                    ui.selectable_value(&mut draft.backlight_off, value, value.label());
                }
            });
    });
    ui.horizontal(|ui| {
        ui.label("Rotation");
        for value in [DisplayDirection::Auto, DisplayDirection::Left, DisplayDirection::Right] {
            ui.selectable_value(&mut draft.display_dir, value, value.label());
        }
    });
}

fn meta_chip(ui: &mut Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("{label}:")).small().color(MUTED));
        ui.label(RichText::new(value).small());
        ui.add_space(10.0);
    });
}
