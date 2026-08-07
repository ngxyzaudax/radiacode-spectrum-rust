use egui::{RichText, Ui};

use crate::settings::state::SettingsState;
use crate::settings::ui_layout::toggle_switch;
use crate::spectrogram::storage::default_spectrograms_dir;
use crate::spectrogram::ui_settings::draw_capture_settings;
use crate::theme::MUTED;

pub fn draw_app_capture(ui: &mut Ui, state: &mut SettingsState, recording: bool) -> bool {
    let mut changed = draw_capture_settings(ui, &mut state.spectrogram, recording);
    changed |= draw_recordings_dir(ui, state);
    changed
}

fn draw_recordings_dir(ui: &mut Ui, state: &mut SettingsState) -> bool {
    let mut changed = false;
    ui.label(RichText::new("Recordings folder").small().color(MUTED));
    ui.horizontal(|ui| {
        let response = ui.add(
            egui::TextEdit::singleline(&mut state.spectrogram.recordings_dir)
                .desired_width(220.0)
                .hint_text(default_spectrograms_dir().display().to_string()),
        );
        changed |= response.changed();
        if ui.button("Browse…").clicked() {
            let start = if state.spectrogram.recordings_dir.trim().is_empty() {
                default_spectrograms_dir()
            } else {
                std::path::PathBuf::from(state.spectrogram.recordings_dir.trim())
            };
            if let Some(path) = rfd::FileDialog::new().set_directory(start).pick_folder() {
                state.spectrogram.recordings_dir = path.display().to_string();
                changed = true;
            }
        }
        if !state.spectrogram.recordings_dir.is_empty()
            && ui.button("Default").on_hover_text("Use the app data folder").clicked()
        {
            state.spectrogram.recordings_dir.clear();
            changed = true;
        }
    });
    if state.spectrogram.recordings_dir.trim().is_empty() {
        ui.label(
            RichText::new(format!("Using {}", default_spectrograms_dir().display()))
                .small()
                .color(MUTED),
        );
    }
    changed
}

pub fn draw_app_polling(ui: &mut Ui, state: &mut SettingsState) -> bool {
    let mut changed = false;
    let mut monitor = state.app.monitor_poll_secs as i32;
    let mut spectrum = state.app.spectrum_refresh_secs as i32;
    changed |= ui
        .add(egui::Slider::new(&mut monitor, 1..=60).text("Monitor (s)"))
        .changed();
    changed |= ui
        .add(egui::Slider::new(&mut spectrum, 1..=60).text("Spectrum (s)"))
        .changed();
    if changed {
        state.app.monitor_poll_secs = monitor as u64;
        state.app.spectrum_refresh_secs = spectrum as u64;
    }
    changed
}

pub fn draw_app_connection(ui: &mut Ui, state: &mut SettingsState) -> bool {
    let mut changed = false;
    changed |= toggle_switch(ui, &mut state.app.remember_device, "Remember last device");
    changed |= toggle_switch(ui, &mut state.app.auto_connect, "Auto-connect on launch");
    if let Some(endpoint) = state.app.last_endpoint.as_ref() {
        ui.label(
            RichText::new(format!(
                "Last device: {} ({})",
                endpoint.address_label(),
                endpoint.transport().label()
            ))
            .small()
            .color(MUTED),
        );
    }
    changed
}

pub fn draw_app_alerts(ui: &mut Ui, state: &mut SettingsState) -> bool {
    toggle_switch(ui, &mut state.app.pc_alarm_repeat, "Beep on alarm")
}
