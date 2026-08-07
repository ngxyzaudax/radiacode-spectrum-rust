use egui::{RichText, Ui};

use radiacode_core::DeviceConfig;

use crate::settings::ui_layout::toggle_switch;
use crate::theme::MUTED;

pub fn draw_signals_panel(ui: &mut Ui, draft: &mut DeviceConfig) {
    ui.label(RichText::new("Masters").small().color(MUTED));
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 20.0;
        toggle_switch(ui, &mut draft.sound_on, "Sound");
        toggle_switch(ui, &mut draft.vibro_on, "Vibration");
        if draft.leds_supported {
            toggle_switch(ui, &mut draft.leds_on, "Light");
        }
    });
    ui.add_space(8.0);
    ui.label(RichText::new("Quantum registration").small().color(MUTED));
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 20.0;
        toggle_switch(ui, &mut draft.sound_ctrl.clicks, "Clicks (sound)");
    });
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 20.0;
        toggle_switch(ui, &mut draft.sound_ctrl.buttons, "Buttons (sound)");
        toggle_switch(ui, &mut draft.vibro_ctrl.buttons, "Buttons (vibrate)");
    });
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 20.0;
        toggle_switch(ui, &mut draft.sound_ctrl.connection, "Connection");
        toggle_switch(ui, &mut draft.sound_ctrl.power, "Power");
    });
}
