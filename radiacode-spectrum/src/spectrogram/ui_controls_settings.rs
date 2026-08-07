use egui::{RichText, Ui};

use crate::spectrogram::state::SpectrogramState;

pub fn draw_overlay_toggles(ui: &mut Ui, state: &mut SpectrogramState) -> bool {
    let mut changed = false;
    ui.label(RichText::new("Overlays").strong());
    changed |= ui.checkbox(&mut state.show_grid, "Grid").changed();
    changed |= ui.checkbox(&mut state.show_count_rate, "Count rate").changed();
    changed |= ui.checkbox(&mut state.show_isotopes, "Isotope lines").changed();
    changed
}
