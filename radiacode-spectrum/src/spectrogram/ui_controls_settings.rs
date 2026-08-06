use egui::{RichText, Ui};

use crate::spectrogram::state::SpectrogramState;

pub fn draw_settings_panel(ui: &mut Ui, state: &mut SpectrogramState) -> bool {
    let mut changed = false;
    ui.label(RichText::new("Capture").strong());
    changed |= ui
        .add(
            egui::Slider::new(&mut state.settings.capture_interval_secs, 1.0..=600.0)
                .logarithmic(false)
                .text("interval (s)"),
        )
        .changed();
    changed |= ui
        .add(
            egui::Slider::new(&mut state.settings.max_samples, 100..=20_000)
                .logarithmic(true)
                .text("max samples"),
        )
        .changed();

    ui.add_space(4.0);
    ui.label(RichText::new("Brightness").strong());
    changed |= ui
        .checkbox(&mut state.settings.auto_brightness, "Auto brightness")
        .changed();
    if !state.settings.auto_brightness {
        changed |= ui
            .add(egui::Slider::new(&mut state.settings.z_min, 0.0..=10_000.0).text("min"))
            .changed();
        changed |= ui
            .add(egui::Slider::new(&mut state.settings.z_max, 1.0..=50_000.0).text("max"))
            .changed();
    }

    ui.add_space(4.0);
    ui.label(RichText::new("Overlays").strong());
    changed |= ui.checkbox(&mut state.show_grid, "Grid").changed();
    changed |= ui.checkbox(&mut state.show_count_rate, "Count rate").changed();
    changed |= ui.checkbox(&mut state.show_isotopes, "Isotope lines").changed();
    changed
}
