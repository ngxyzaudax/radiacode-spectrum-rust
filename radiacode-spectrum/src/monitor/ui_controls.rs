use egui::{RichText, Ui};

use crate::theme::MUTED;

pub fn draw_monitor_controls(ui: &mut Ui) {
    ui.add_space(8.0);
    ui.separator();
    ui.add_space(8.0);
    ui.label(
        RichText::new("Alarm thresholds and units are edited in the Settings tab.")
            .small()
            .color(MUTED),
    );
}
