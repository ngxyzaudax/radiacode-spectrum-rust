use egui::{RichText, Ui};

use crate::analysis::spectrum::CollapsedSpectrum;
use crate::analysis::state::AnalysisState;
use crate::theme::{analysis_sample_color, ANALYSIS_BACKGROUND, MUTED};

pub fn draw_background_card(ui: &mut Ui, spectrum: Option<&CollapsedSpectrum>) {
    ui.label(RichText::new("Background").strong().color(ANALYSIS_BACKGROUND));
    draw_spectrum_summary(ui, spectrum);
}

pub fn draw_samples_card(ui: &mut Ui, state: &mut AnalysisState) {
    ui.label(RichText::new("Samples").strong());
    if state.samples.is_empty() {
        ui.label(RichText::new("None selected").small().color(MUTED));
        return;
    }
    let mut remove_index = None;
    for (index, sample) in state.samples.iter().enumerate() {
        let color = analysis_sample_color(index);
        ui.horizontal(|ui| {
            ui.label(RichText::new("●").color(color));
            ui.vertical(|ui| {
                ui.label(RichText::new(&sample.spectrum.name).small().color(color));
                draw_spectrum_meta(ui, &sample.spectrum);
            });
            if ui.small_button("×").on_hover_text("Remove sample").clicked() {
                remove_index = Some(index);
            }
        });
        ui.add_space(4.0);
    }
    if let Some(index) = remove_index {
        state.remove_sample_at(index);
    }
}

pub fn draw_warnings(ui: &mut Ui, state: &AnalysisState) {
    if state.samples.iter().any(|sample| {
        sample
            .comparison
            .as_ref()
            .is_some_and(|comparison| comparison.calib_warning)
    }) {
        ui.label(
            RichText::new(
                "Calibration differs for one or more samples; plots use the first sample axis.",
            )
            .small()
            .color(egui::Color32::from_rgb(220, 180, 80)),
        );
    }
}

fn draw_spectrum_summary(ui: &mut Ui, spectrum: Option<&CollapsedSpectrum>) {
    let Some(spectrum) = spectrum else {
        ui.label(RichText::new("Not selected").small().color(MUTED));
        return;
    };
    ui.label(RichText::new(&spectrum.name).small().color(ANALYSIS_BACKGROUND));
    draw_spectrum_meta(ui, spectrum);
}

fn draw_spectrum_meta(ui: &mut Ui, spectrum: &CollapsedSpectrum) {
    ui.label(
        RichText::new(format!(
            "Serial: {}",
            spectrum.device_serial.as_deref().unwrap_or("-")
        ))
        .small()
        .color(MUTED),
    );
    ui.label(
        RichText::new(format!(
            "Channels: {} | Live: {:.1}s | Counts: {}",
            spectrum.channel_count, spectrum.live_time_secs, spectrum.total_counts
        ))
        .small()
        .color(MUTED),
    );
}
