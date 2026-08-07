use egui::{CollapsingHeader, RichText, ScrollArea, Ui};

use crate::model::{ConnectionState, DeviceInfo};
use crate::settings::action::SettingsAction;
use crate::settings::state::{SettingsDeviceOp, SettingsState};
use crate::settings::ui_columns::{draw_application_column, draw_detector_column};
use crate::settings::ui_toolbar::draw_sticky_toolbar;

pub fn draw_settings_view(
    ui: &mut Ui,
    state: &mut SettingsState,
    connection: ConnectionState,
    device_info: Option<&DeviceInfo>,
    recording: bool,
) -> Option<SettingsAction> {
    let connected = connection == ConnectionState::Connected;
    let editing = state.device_op == SettingsDeviceOp::Idle;
    let mut action = None;
    if state.show_load_confirm {
        draw_load_confirm_dialog(ui, state, &mut action);
        if action.is_some() {
            return action;
        }
    }
    if let Some(next) = draw_sticky_toolbar(ui, state, connected) {
        action = Some(next);
    }
    ui.add_space(6.0);
    ui.separator();
    ScrollArea::vertical()
        .id_salt("settings_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            CollapsingHeader::new(RichText::new("Detector").strong().size(15.0))
                .default_open(true)
                .show(ui, |ui| {
                    ui.add_space(4.0);
                    draw_detector_column(ui, state, connected, editing, device_info, &mut action);
                });
            ui.add_space(8.0);
            CollapsingHeader::new(RichText::new("Application").strong().size(15.0))
                .default_open(true)
                .show(ui, |ui| {
                    ui.add_space(4.0);
                    draw_application_column(ui, state, recording, &mut action);
                });
        });
    action
}

fn draw_load_confirm_dialog(ui: &mut Ui, state: &mut SettingsState, action: &mut Option<SettingsAction>) {
    egui::Window::new("Unsaved changes")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ui.ctx(), |ui| {
            ui.label("You have unsaved changes. Load from device and discard your edits?");
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Load anyway").clicked() {
                    *action = Some(SettingsAction::ConfirmLoad);
                }
                if ui.button("Keep editing").clicked() {
                    state.show_load_confirm = false;
                    *action = Some(SettingsAction::CancelLoad);
                }
            });
        });
}
