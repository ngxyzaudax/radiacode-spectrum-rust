use egui::{CollapsingHeader, RichText, Ui};

use crate::spectrogram::color_scheme::ColorScheme;
use crate::spectrogram::settings::SpectrogramSettings;
use crate::spectrogram::state::SpectrogramState;
use crate::theme::MUTED;

pub fn draw_spectrogram_settings(ui: &mut Ui, state: &mut SpectrogramState) -> bool {
    let mut changed = false;
    let recording = state.is_recording();
    CollapsingHeader::new(RichText::new("Capture").strong())
        .default_open(false)
        .show(ui, |ui| {
            changed |= draw_capture_controls(ui, &mut state.settings, recording);
        });
    ui.add_space(2.0);
    CollapsingHeader::new(RichText::new("Display").strong())
        .default_open(false)
        .show(ui, |ui| {
            changed |= draw_display_controls(ui, &mut state.settings);
            changed |= draw_overlay_controls(ui, state);
        });
    changed
}

pub fn draw_capture_settings(
    ui: &mut Ui,
    settings: &mut SpectrogramSettings,
    recording: bool,
) -> bool {
    let mut changed = draw_capture_controls(ui, settings, recording);
    changed |= draw_display_controls(ui, settings);
    changed
}

pub fn draw_capture_controls(
    ui: &mut Ui,
    settings: &mut SpectrogramSettings,
    recording: bool,
) -> bool {
    let mut changed = false;
    ui.add_enabled_ui(!recording, |ui| {
        changed |= ui
            .add(
                egui::Slider::new(&mut settings.capture_interval_secs, 1.0..=600.0)
                    .text("Interval (s)"),
            )
            .changed();
        changed |= ui
            .add(
                egui::Slider::new(&mut settings.max_samples, 100..=20_000)
                    .logarithmic(true)
                    .text("Max samples"),
            )
            .changed();
    });
    if recording {
        ui.label(
            RichText::new("Interval and max samples are locked while recording.")
                .small()
                .color(MUTED),
        );
    }
    changed
}

pub fn draw_display_controls(ui: &mut Ui, settings: &mut SpectrogramSettings) -> bool {
    let mut changed = false;
    changed |= ui
        .checkbox(&mut settings.auto_brightness, "Auto brightness")
        .changed();
    if !settings.auto_brightness {
        changed |= ui
            .add(egui::Slider::new(&mut settings.z_min, 0.0..=10_000.0).text("Z min"))
            .changed();
        changed |= ui
            .add(egui::Slider::new(&mut settings.z_max, 1.0..=50_000.0).text("Z max"))
            .changed();
    }
    ui.horizontal(|ui| {
        ui.label(RichText::new("Palette").small().color(MUTED));
        for palette in ColorScheme::ALL {
            changed |= ui
                .selectable_value(&mut settings.palette, palette, palette.label())
                .changed();
        }
    });
    changed
}

fn draw_overlay_controls(ui: &mut Ui, state: &mut SpectrogramState) -> bool {
    ui.add_space(4.0);
    ui.label(RichText::new("Overlays").small().color(MUTED));
    let mut changed = false;
    changed |= ui.checkbox(&mut state.show_grid, "Grid").changed();
    changed |= ui.checkbox(&mut state.show_count_rate, "Count rate").changed();
    changed |= ui.checkbox(&mut state.show_isotopes, "Isotope lines").changed();
    changed
}
