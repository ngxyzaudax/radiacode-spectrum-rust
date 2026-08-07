use egui::{RichText, Ui};

use crate::theme::MUTED;

pub fn draw_recording_search(
    ui: &mut Ui,
    filter: &mut String,
    matched_count: usize,
    total_count: usize,
) {
    egui::Frame::new()
        .fill(ui.visuals().extreme_bg_color)
        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
        .corner_radius(4)
        .inner_margin(egui::Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Search").small().strong().color(MUTED));
                let _response = ui.add(
                    egui::TextEdit::singleline(filter)
                        .hint_text("Name, comment, or serial…")
                        .desired_width(ui.available_width() - 52.0),
                );
                if !filter.is_empty() && ui.small_button("Clear").clicked() {
                    filter.clear();
                } else if filter.is_empty() {
                    ui.add_space(ui.spacing().interact_size.x);
                }
            });
        });
    ui.add_space(4.0);
    draw_recording_count(ui, filter, matched_count, total_count);
}

fn draw_recording_count(ui: &mut Ui, filter: &str, matched_count: usize, total_count: usize) {
    let text = if filter.trim().is_empty() {
        if total_count == 0 {
            "No recordings".to_string()
        } else if total_count == 1 {
            "1 recording".to_string()
        } else {
            format!("{total_count} recordings")
        }
    } else if matched_count == 0 {
        format!("No matches in {total_count} recordings")
    } else {
        format!("{matched_count} of {total_count} recordings")
    };
    ui.label(RichText::new(text).small().color(MUTED));
}
