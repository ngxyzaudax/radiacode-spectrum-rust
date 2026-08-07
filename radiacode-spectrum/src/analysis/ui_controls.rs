use egui::{RichText, Ui};

use crate::analysis::state::AnalysisState;
use crate::analysis::ui_role_cards::{draw_background_card, draw_samples_card, draw_warnings};
use crate::scale::YScale;
use crate::smooth::normalize_window;
use crate::spectrogram::model::RecordingEntry;
use crate::theme::{analysis_sample_color, MUTED};
use crate::ui_recording_library::{
    analysis_name_color, draw_analysis_role_badges, draw_empty_library, draw_recording_card,
    draw_recording_meta, draw_recording_search, draw_recording_title, scroll_recording_list,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalysisAction {
    ClearSelection,
}

pub fn draw_analysis_controls(
    ui: &mut Ui,
    state: &mut AnalysisState,
    y_scale: &mut YScale,
) -> Option<AnalysisAction> {
    ui.label(RichText::new("Library").strong());
    ui.add_space(4.0);
    draw_library_list(ui, state);
    ui.add_space(8.0);
    ui.label(RichText::new("Y axis").strong());
    ui.horizontal(|ui| {
        ui.selectable_value(y_scale, YScale::Linear, "Linear");
        ui.selectable_value(y_scale, YScale::Logarithmic, "Log");
    });
    ui.horizontal(|ui| {
        ui.selectable_value(&mut state.outline_only, false, "Filled");
        ui.selectable_value(&mut state.outline_only, true, "Outline");
    });
    ui.add_space(4.0);
    ui.label("Smooth window (channels)");
    let mut slider = state.smooth_window.clamp(1, 16) as i32;
    if ui
        .add(egui::Slider::new(&mut slider, 1..=16).text("channels"))
        .changed()
    {
        state.smooth_window = normalize_window(slider as usize);
    }
    ui.add_space(8.0);
    draw_background_card(ui, state.background.as_ref());
    ui.add_space(6.0);
    draw_samples_card(ui, state);
    draw_warnings(ui, state);
    if ui.button("Clear selection").clicked() {
        return Some(AnalysisAction::ClearSelection);
    }
    if !state.status.is_empty() {
        ui.label(RichText::new(&state.status).small().color(MUTED));
    }
    if !state.error.is_empty() {
        ui.label(RichText::new(&state.error).small().color(egui::Color32::from_rgb(220, 120, 120)));
    }
    None
}

fn draw_library_list(ui: &mut Ui, state: &mut AnalysisState) {
    let total_count = state.library.len();
    let entries: Vec<RecordingEntry> = state.filtered_library();
    draw_recording_search(ui, &mut state.library_filter, entries.len(), total_count);
    ui.add_space(4.0);
    if entries.is_empty() {
        draw_empty_library(ui, state.library_filter.trim().is_empty());
        return;
    }
    scroll_recording_list(ui, 260.0, |ui| {
        for entry in entries {
            draw_library_entry(ui, state, &entry);
        }
    });
}

fn draw_library_entry(ui: &mut Ui, state: &mut AnalysisState, entry: &RecordingEntry) {
    let is_bg = state.is_background(&entry.path);
    let sample_index = state.sample_index(&entry.path);
    let sample_tint = sample_index.map(analysis_sample_color);
    draw_recording_card(ui, |ui| {
        ui.horizontal(|ui| {
            draw_recording_title(ui, &entry.name, analysis_name_color(is_bg, sample_tint));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                draw_analysis_role_badges(ui, is_bg, sample_tint);
            });
        });
        draw_recording_meta(ui, entry);
        if !entry.comment.is_empty() {
            ui.label(RichText::new(&entry.comment).small().italics().color(MUTED));
        }
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.add_enabled_ui(sample_tint.is_none(), |ui| {
                if ui.selectable_label(is_bg, "Background").clicked() && !is_bg {
                    state.set_background(entry);
                }
            });
            ui.add_enabled_ui(!is_bg, |ui| {
                let selected = sample_tint.is_some();
                if ui.selectable_label(selected, "Sample").clicked() {
                    state.toggle_sample(entry);
                }
            });
        });
    });
}
