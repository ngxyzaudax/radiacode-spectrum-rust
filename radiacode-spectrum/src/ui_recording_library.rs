use egui::{Color32, RichText, Ui};

use crate::spectrogram::model::RecordingEntry;
use crate::theme::{ANALYSIS_BACKGROUND, MUTED};

pub use crate::ui_recording_search::draw_recording_search;

pub fn recording_meta_line(entry: &RecordingEntry) -> String {
    format!(
        "{} rows · {:.0}s · {} ch · {}",
        entry.row_count,
        entry.interval_secs,
        entry.channel_count,
        entry.device_serial.as_deref().unwrap_or("—")
    )
}

pub fn draw_recording_card(ui: &mut Ui, body: impl FnOnce(&mut Ui)) {
    ui.add_space(4.0);
    egui::Frame::new()
        .fill(ui.visuals().faint_bg_color)
        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
        .corner_radius(6)
        .inner_margin(8)
        .show(ui, body);
}

pub fn draw_recording_title(ui: &mut Ui, name: &str, accent: Option<Color32>) {
    ui.label(
        RichText::new(name)
            .strong()
            .color(accent.unwrap_or(Color32::WHITE)),
    );
}

pub fn draw_recording_meta(ui: &mut Ui, entry: &RecordingEntry) {
    ui.label(RichText::new(recording_meta_line(entry)).small().color(MUTED));
}

pub fn draw_recording_comment(ui: &mut Ui, comment: &str, on_add: impl FnOnce()) {
    if comment.is_empty() {
        if ui
            .link(RichText::new("Add comment…").small().color(MUTED))
            .clicked()
        {
            on_add();
        }
        return;
    }
    ui.label(RichText::new(comment).small().italics().color(MUTED));
}

pub fn draw_role_badge(ui: &mut Ui, label: &str, color: Color32) {
    egui::Frame::new()
        .fill(color.gamma_multiply(0.25))
        .stroke(egui::Stroke::new(1.0, color))
        .corner_radius(3)
        .inner_margin(egui::Margin::symmetric(6, 2))
        .show(ui, |ui| {
            ui.label(RichText::new(label).small().strong().color(color));
        });
}

pub fn draw_analysis_role_badges(
    ui: &mut Ui,
    is_background: bool,
    sample_color: Option<Color32>,
) {
    ui.horizontal(|ui| {
        if is_background {
            draw_role_badge(ui, "Background", ANALYSIS_BACKGROUND);
        }
        if let Some(color) = sample_color {
            draw_role_badge(ui, "Sample", color);
        }
    });
}

pub fn analysis_name_color(
    is_background: bool,
    sample_color: Option<Color32>,
) -> Option<Color32> {
    sample_color.or(is_background.then_some(ANALYSIS_BACKGROUND))
}

pub fn draw_empty_library(ui: &mut Ui, filter_empty: bool) {
    let message = if filter_empty {
        "No saved recordings yet. Record or import one on the Spectrogram tab."
    } else {
        "No recordings match your search."
    };
    egui::Frame::new()
        .fill(ui.visuals().extreme_bg_color)
        .corner_radius(6)
        .inner_margin(12)
        .show(ui, |ui| {
            ui.label(RichText::new(message).small().weak());
        });
}

pub fn scroll_recording_list(ui: &mut Ui, max_height: f32, body: impl FnOnce(&mut Ui)) {
    egui::ScrollArea::vertical()
        .max_height(max_height)
        .auto_shrink([false, false])
        .show(ui, body);
}
