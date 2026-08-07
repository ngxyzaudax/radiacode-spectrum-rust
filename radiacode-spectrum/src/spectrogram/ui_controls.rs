use std::path::PathBuf;

use egui::{RichText, Ui};

use crate::model::ConnectionState;
use crate::theme::{ACCENT, MUTED};
use crate::spectrogram::library;
use crate::spectrogram::model::RecordingEntry;
use crate::spectrogram::state::SpectrogramState;
use crate::spectrogram::ui_controls_settings::draw_overlay_toggles;

pub enum SpectrogramControlsAction {
    StartRecording,
    StopRecording,
    ResumeRecording,
    CloseLoaded,
    Load(PathBuf),
    SettingsChanged,
    LibraryChanged,
}

pub fn draw_spectrogram_controls(
    ui: &mut Ui,
    state: &mut SpectrogramState,
    connection: ConnectionState,
    busy: bool,
) -> Option<SpectrogramControlsAction> {
    let mut action = None;
    ui.add_space(8.0);
    ui.separator();
    ui.add_space(8.0);
    ui.label(RichText::new("Spectrogram").strong());
    ui.add_space(6.0);
    action = draw_transport(ui, state, connection).or(action);

    ui.add_space(4.0);
    ui.label(
        RichText::new("Capture interval, palette, and brightness are in Settings.")
            .small()
            .color(MUTED),
    );
    ui.add_space(4.0);
    let mut settings_changed = draw_overlay_toggles(ui, state);

    ui.add_space(4.0);
    if ui.button("Reset accumulation").clicked() {
        state.reset_accumulation();
    }

    if settings_changed {
        action = Some(SpectrogramControlsAction::SettingsChanged);
    }

    draw_status(ui, state);
    ui.add_space(8.0);
    ui.separator();
    ui.add_space(6.0);
    ui.label(RichText::new("Spectrogram Library").strong());
    ui.add_space(4.0);
    draw_library(ui, state, busy, &mut action);
    action
}

fn draw_transport(
    ui: &mut Ui,
    state: &SpectrogramState,
    connection: ConnectionState,
) -> Option<SpectrogramControlsAction> {
    let recording = state.is_recording();
    let mut action = None;
    ui.horizontal(|ui| {
        if recording {
            ui.label(RichText::new("Recording").color(ACCENT));
            if ui.button("Stop").clicked() {
                action = Some(SpectrogramControlsAction::StopRecording);
            }
        } else if ui
            .add_enabled(
                connection == ConnectionState::Connected,
                egui::Button::new("Record"),
            )
            .clicked()
        {
            action = Some(SpectrogramControlsAction::StartRecording);
        } else if state.paused_recording_path.is_some()
            && ui
                .add_enabled(connection == ConnectionState::Connected, egui::Button::new("Resume"))
                .clicked()
        {
            action = Some(SpectrogramControlsAction::ResumeRecording);
        }
    });
    if state.is_viewing_library() {
        ui.horizontal(|ui| {
            if ui.button("Close spectrogram").clicked() {
                action = Some(SpectrogramControlsAction::CloseLoaded);
            }
        });
    }
    action
}

fn draw_status(ui: &mut Ui, state: &SpectrogramState) {
    ui.add_space(4.0);
    ui.label(
        RichText::new(format!(
            "History: {} rows ({:.0}s)",
            state.live_row_count(),
            state
                .live_series
                .as_ref()
                .map(|series| series.duration_secs())
                .unwrap_or(0.0)
        ))
        .small()
        .color(MUTED),
    );
    if let Some(series) = state.active_series() {
        if let Some(row) = series.rows.last() {
            let last_total: u64 = row.counts.iter().map(|&value| value as u64).sum();
            ui.label(
                RichText::new(format!("Last row total: {last_total} counts"))
                    .small()
                    .color(MUTED),
            );
        }
    }
    if !state.status.is_empty() {
        ui.add_space(4.0);
        ui.label(RichText::new(&state.status).small().color(MUTED));
    }
}

fn draw_library_editor(
    ui: &mut Ui,
    state: &mut SpectrogramState,
    action: &mut Option<SpectrogramControlsAction>,
) {
    let Some(path) = state.library_edit_path.clone() else {
        return;
    };
    ui.separator();
    ui.label(RichText::new("Edit library entry").strong());
    ui.text_edit_singleline(&mut state.library_edit_name);
    ui.text_edit_singleline(&mut state.library_edit_comment);
    ui.horizontal(|ui| {
        if ui.button("Save").clicked() {
            let _ = library::rename_entry(&path, &state.library_edit_name);
            let _ = library::set_comment(&path, &state.library_edit_comment);
            state.library_edit_path = None;
            state.refresh_history();
            *action = Some(SpectrogramControlsAction::LibraryChanged);
        }
        if ui.button("Cancel").clicked() {
            state.library_edit_path = None;
        }
    });
}

fn draw_library(
    ui: &mut Ui,
    state: &mut SpectrogramState,
    busy: bool,
    action: &mut Option<SpectrogramControlsAction>,
) {
    ui.text_edit_singleline(&mut state.library_filter);
    draw_library_editor(ui, state, action);
    if ui.button("Import .rcspg").clicked() {
        if let Some(path) = rfd::FileDialog::new().add_filter("rcspg", &["rcspg"]).pick_file() {
            match library::import_rcspg(&path) {
                Ok(saved) => {
                    state.status = format!("Imported {}", saved.display());
                    state.refresh_history();
                    *action = Some(SpectrogramControlsAction::LibraryChanged);
                }
                Err(message) => state.status = message,
            }
        }
    }
    let entries: Vec<RecordingEntry> = state.filtered_history();
    if entries.is_empty() {
        ui.label(RichText::new("No saved spectrograms yet.").weak().small());
        return;
    }
    egui::ScrollArea::vertical()
        .max_height(240.0)
        .show(ui, |ui| {
            for entry in entries {
                draw_library_entry(ui, state, busy, &entry, action);
            }
        });
}

fn draw_library_entry(
    ui: &mut Ui,
    state: &mut SpectrogramState,
    busy: bool,
    entry: &RecordingEntry,
    action: &mut Option<SpectrogramControlsAction>,
) {
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        if ui
            .add_enabled(
                !busy,
                egui::Button::new(&entry.name).min_size([ui.available_width() - 160.0, 0.0].into()),
            )
            .clicked()
        {
            *action = Some(SpectrogramControlsAction::Load(entry.path.clone()));
        }
        if ui.small_button("Ren").clicked() {
            state.library_edit_path = Some(entry.path.clone());
            state.library_edit_name = entry.name.clone();
            state.library_edit_comment = entry.comment.clone();
        }
        if ui.small_button("Exp").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .set_file_name(format!("{}.rcspg", entry.name))
                .save_file()
            {
                if let Err(message) = library::export_rcspg(&entry.path, &path) {
                    state.status = message;
                } else {
                    state.status = format!("Exported {}", path.display());
                }
            }
        }
        if ui.small_button("Del").clicked() {
            if library::delete_entry(&entry.path).is_ok() {
                state.refresh_history();
                *action = Some(SpectrogramControlsAction::LibraryChanged);
            }
        }
    });
    ui.label(
        RichText::new(format!(
            "{} rows | {:.0}s interval | {}",
            entry.row_count,
            entry.interval_secs,
            entry.device_serial.as_deref().unwrap_or("-")
        ))
        .small()
        .color(MUTED),
    );
}
