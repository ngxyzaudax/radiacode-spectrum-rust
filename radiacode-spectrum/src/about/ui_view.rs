use egui::{RichText, Ui};

use crate::theme::MUTED;

const APP_NAME: &str = "Radiacode Spectrum";
const AUTHOR: &str = "Nikola Galiot";
const EMAIL: &str = "nikola.galiot@protonmail.com";
const REPO_URL: &str = "https://github.com/ngxyzaudax/radiacode-spectrum-rust";

pub fn draw_about_view(ui: &mut Ui) {
    ui.add_space(24.0);
    ui.horizontal(|ui| {
        ui.add_space(24.0);
        ui.vertical(|ui| {
            ui.label(RichText::new(APP_NAME).size(20.0).strong());
            ui.label(
                RichText::new(format!("Version {}", env!("CARGO_PKG_VERSION")))
                    .small()
                    .color(MUTED),
            );
            ui.add_space(12.0);
            ui.label("Desktop spectrogram and spectrum analysis for RadiaCode detectors.");
            ui.add_space(16.0);
            ui.label(RichText::new("Author").strong());
            ui.label(AUTHOR);
            ui.hyperlink_to(EMAIL, format!("mailto:{EMAIL}"));
            ui.add_space(12.0);
            ui.label(RichText::new("Source").strong());
            ui.hyperlink_to(REPO_URL, REPO_URL);
            ui.add_space(16.0);
            ui.label(
                RichText::new("Licensed under the GNU Affero General Public License v3.0.")
                    .small()
                    .color(MUTED),
            );
        });
    });
}
