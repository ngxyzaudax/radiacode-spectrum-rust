use egui::Ui;

use radiacode_core::{count_unit_label, dose_unit_label, AlarmSignalMode, DeviceConfig};

use crate::settings::ui_alarm_card::alarm_card;

pub fn draw_alarms_panel(ui: &mut Ui, draft: &mut DeviceConfig) {
    let dose_unit = dose_unit_label(draft.alarms.dose_unit_sv);
    let count_unit = count_unit_label(draft.alarms.count_unit_cpm);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        dose_rate_card(ui, draft, dose_unit);
        count_rate_card(ui, draft, count_unit);
        accum_dose_card(ui, draft, dose_unit);
    });
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label("Signal mode");
        ui.selectable_value(&mut draft.alarm_mode, AlarmSignalMode::Once, "Once");
        ui.selectable_value(
            &mut draft.alarm_mode,
            AlarmSignalMode::Continuous,
            "Continuous",
        );
    });
}

fn dose_rate_card(ui: &mut Ui, draft: &mut DeviceConfig, unit: &str) {
    alarm_card(
        ui,
        "Dose rate",
        &mut draft.alarms.l1_dose_rate,
        &mut draft.alarms.l2_dose_rate,
        unit,
        0.01,
        [
            (
                &mut draft.sound_ctrl.dose_rate_alarm1,
                &mut draft.vibro_ctrl.dose_rate_alarm1,
            ),
            (
                &mut draft.sound_ctrl.dose_rate_alarm2,
                &mut draft.vibro_ctrl.dose_rate_alarm2,
            ),
            (
                &mut draft.sound_ctrl.dose_rate_out_of_scale,
                &mut draft.vibro_ctrl.dose_rate_out_of_scale,
            ),
        ],
    );
}

fn count_rate_card(ui: &mut Ui, draft: &mut DeviceConfig, unit: &str) {
    alarm_card(
        ui,
        "Count rate",
        &mut draft.alarms.l1_count_rate,
        &mut draft.alarms.l2_count_rate,
        unit,
        1.0,
        [
            (
                &mut draft.sound_ctrl.count_rate_alarm1,
                &mut draft.vibro_ctrl.count_rate_alarm1,
            ),
            (
                &mut draft.sound_ctrl.count_rate_alarm2,
                &mut draft.vibro_ctrl.count_rate_alarm2,
            ),
            (
                &mut draft.sound_ctrl.count_rate_out_of_scale,
                &mut draft.vibro_ctrl.count_rate_out_of_scale,
            ),
        ],
    );
}

fn accum_dose_card(ui: &mut Ui, draft: &mut DeviceConfig, unit: &str) {
    alarm_card(
        ui,
        "Accum. dose",
        &mut draft.alarms.l1_dose,
        &mut draft.alarms.l2_dose,
        unit,
        1.0,
        [
            (
                &mut draft.sound_ctrl.dose_alarm1,
                &mut draft.vibro_ctrl.dose_alarm1,
            ),
            (
                &mut draft.sound_ctrl.dose_alarm2,
                &mut draft.vibro_ctrl.dose_alarm2,
            ),
            (
                &mut draft.sound_ctrl.dose_out_of_scale,
                &mut draft.vibro_ctrl.dose_out_of_scale,
            ),
        ],
    );
}
