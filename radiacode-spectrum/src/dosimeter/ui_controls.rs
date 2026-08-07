use egui::{RichText, Ui};

use crate::theme::MUTED;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DosimeterAction {
    ResetDose,
}

pub fn draw_dosimeter_controls(ui: &mut Ui) -> Option<DosimeterAction> {
    ui.label(
        RichText::new("Accumulated dose thresholds and units are edited in Settings.")
            .small()
            .color(MUTED),
    );
    ui.add_space(8.0);
    let confirm_id = ui.id().with("dose_reset_confirm");
    let confirming = ui.data_mut(|data| *data.get_temp_mut_or(confirm_id, false));
    if confirming {
        ui.label("Reset accumulated dose on device?");
        let mut action = None;
        ui.horizontal(|ui| {
            if ui.button("Confirm reset").clicked() {
                ui.data_mut(|data| data.insert_temp(confirm_id, false));
                action = Some(DosimeterAction::ResetDose);
            }
            if ui.button("Cancel").clicked() {
                ui.data_mut(|data| data.insert_temp(confirm_id, false));
            }
        });
        return action;
    }
    if ui.button("Reset dose").clicked() {
        ui.data_mut(|data| data.insert_temp(confirm_id, true));
    }
    None
}
