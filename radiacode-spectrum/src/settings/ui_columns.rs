use egui::{RichText, Ui};

use crate::model::DeviceInfo;
use crate::settings::action::SettingsAction;
use crate::settings::state::SettingsState;
use crate::settings::ui_app::{draw_app_alerts, draw_app_capture, draw_app_connection, draw_app_polling};
use crate::settings::ui_device::{
    draw_alarms_panel, draw_device_info, draw_screen_panel, draw_signals_panel, draw_units_panel,
};
use crate::settings::ui_layout::settings_section;
use crate::theme::MUTED;

pub fn draw_detector_column(
    ui: &mut Ui,
    state: &mut SettingsState,
    connected: bool,
    editing: bool,
    device_info: Option<&DeviceInfo>,
    action: &mut Option<SettingsAction>,
) {
    settings_section(ui, "Device", "Live status from the connected detector.", |ui| {
        draw_device_info(ui, device_info);
    });
    if let Some(draft) = state.draft.as_mut() {
        if connected {
            ui.add_enabled_ui(editing, |ui| {
                settings_section(ui, "Units", "", |ui| {
                    draw_units_panel(ui, draft);
                });
                settings_section(ui, "Alarms", "Warning / danger thresholds and signal mode.", |ui| {
                    draw_alarms_panel(ui, draft);
                });
                settings_section(ui, "Screen", "Brightness, timeout, and rotation.", |ui| {
                    draw_screen_panel(ui, draft);
                });
                settings_section(ui, "Signals", "Masters, clicks, and per-event flags.", |ui| {
                    draw_signals_panel(ui, draft);
                });
            });
            if editing {
                ui.add_space(6.0);
                if ui
                    .button("Sync clock from PC")
                    .on_hover_text("Set the detector clock to this computer's local time")
                    .clicked()
                {
                    *action = Some(SettingsAction::SyncClock);
                }
            }
        }
    } else if connected {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(RichText::new("Loading from device…").color(MUTED));
        });
    } else {
        ui.label(
            RichText::new("Connect a device to view and edit detector settings.").color(MUTED),
        );
    }
}

pub fn draw_application_column(
    ui: &mut Ui,
    state: &mut SettingsState,
    action: &mut Option<SettingsAction>,
) {
    ui.label(
        RichText::new("Stored on this PC only. Changes save immediately.")
            .small()
            .color(MUTED),
    );
    ui.add_space(6.0);
    settings_section(ui, "Spectrogram capture", "Saved locally on this PC.", |ui| {
        if draw_app_capture(ui, state) {
            *action = Some(SettingsAction::SpectrogramChanged);
        }
    });
    settings_section(ui, "Polling", "How often live data is refreshed.", |ui| {
        if draw_app_polling(ui, state) {
            *action = Some(SettingsAction::AppChanged);
        }
    });
    settings_section(ui, "Connection", "Startup and remembered devices.", |ui| {
        if draw_app_connection(ui, state) {
            *action = Some(SettingsAction::AppChanged);
        }
    });
    settings_section(ui, "PC alerts", "Repeat detector alarms on this computer.", |ui| {
        if draw_app_alerts(ui, state) {
            *action = Some(SettingsAction::AppChanged);
        }
    });
}
