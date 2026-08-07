use egui::{RichText, Ui};

use crate::settings::action::SettingsAction;
use crate::settings::state::{SettingsDeviceOp, SettingsState};
use crate::theme::{ACCENT, MUTED};

pub fn draw_sticky_toolbar(
    ui: &mut Ui,
    state: &SettingsState,
    connected: bool,
) -> Option<SettingsAction> {
    let mut action = None;
    let dirty = state.draft_dirty();
    let op_busy = state.device_busy();
    ui.horizontal(|ui| {
        draw_status(ui, state, connected, dirty);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if connected
                && dirty
                && ui
                    .add_enabled(!op_busy, egui::Button::new("Discard"))
                    .on_hover_text("Revert to last loaded settings")
                    .clicked()
            {
                action = Some(SettingsAction::DiscardChanges);
            }
            if connected
                && ui
                    .add_enabled(!op_busy && dirty, egui::Button::new("Save to device"))
                    .on_hover_text("Write edited settings to the detector")
                    .clicked()
            {
                action = Some(SettingsAction::SaveDevice);
            }
            if connected
                && ui
                    .add_enabled(!op_busy, egui::Button::new("Load from device"))
                    .on_hover_text("Read current settings from the detector")
                    .clicked()
            {
                action = Some(SettingsAction::LoadDevice);
            }
        });
    });
    action
}

fn draw_status(ui: &mut Ui, state: &SettingsState, connected: bool, dirty: bool) {
    match state.device_op {
        SettingsDeviceOp::Loading => {
            ui.spinner();
            ui.label(RichText::new("Loading from device…").color(MUTED));
        }
        SettingsDeviceOp::Saving => {
            ui.spinner();
            ui.label(RichText::new("Saving to device…").color(MUTED));
        }
        SettingsDeviceOp::Idle if !state.status.is_empty() => {
            ui.label(RichText::new(&state.status).small().color(MUTED));
        }
        SettingsDeviceOp::Idle if dirty => {
            ui.label(RichText::new("Unsaved changes").small().color(ACCENT));
        }
        SettingsDeviceOp::Idle if connected && state.draft.is_some() => {
            ui.label(RichText::new("Ready").small().color(MUTED));
        }
        SettingsDeviceOp::Idle if !connected => {
            ui.label(RichText::new("Not connected").small().color(MUTED));
        }
        SettingsDeviceOp::Idle => {}
    }
}
