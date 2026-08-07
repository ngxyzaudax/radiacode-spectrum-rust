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
    let mut confirm = ui.data_mut(|data| *data.get_temp_mut_or(confirm_id, false));
    if confirm {
        ui.label("Reset accumulated dose on device?");
        let mut reset = false;
        let mut cancel = false;
        ui.horizontal(|ui| {
            reset = ui.button("Confirm reset").clicked();
            cancel = ui.button("Cancel").clicked();
        });
        if reset {
            confirm = false;
            ui.data_mut(|data| data.insert_temp(confirm_id, false));
            return Some(DosimeterAction::ResetDose);
        }
        if cancel {
            confirm = false;
            ui.data_mut(|data| data.insert_temp(confirm_id, false));
        }
        ui.data_mut(|data| data.insert_temp(confirm_id, confirm));
        return None;
    }
    if ui.button("Reset dose").clicked() {
        confirm = true;
        ui.data_mut(|data| data.insert_temp(confirm_id, true));
    }
    None
}
