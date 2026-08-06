use egui::{RichText, Ui};

use crate::model::ConnectionState;
use crate::scale::YScale;
use crate::smooth::normalize_window;

pub struct ControlsProps<'a> {
    pub connection: ConnectionState,
    pub live: &'a mut bool,
    pub y_scale: &'a mut YScale,
    pub smooth_window: &'a mut usize,
}

pub enum ControlsAction {
    Reset,
}

pub fn draw_spectrum_controls(ui: &mut Ui, props: ControlsProps<'_>) -> Option<ControlsAction> {
    if props.connection != ConnectionState::Connected {
        return None;
    }

    let mut action = None;
    ui.add_space(8.0);
    ui.separator();
    ui.add_space(8.0);
    ui.label(RichText::new("Spectrum").strong());
    ui.checkbox(props.live, "Live refresh");
    ui.add_space(4.0);
    ui.label("Y scale");
    ui.horizontal(|ui| {
        ui.selectable_value(props.y_scale, YScale::Linear, "Linear");
        ui.selectable_value(props.y_scale, YScale::Logarithmic, "Log");
    });
    ui.add_space(4.0);
    ui.label("Smooth window (channels)");
    let mut slider = (*props.smooth_window).clamp(1, 16) as i32;
    if ui
        .add(egui::Slider::new(&mut slider, 1..=16).text("channels"))
        .changed()
    {
        *props.smooth_window = normalize_window(slider as usize);
    }
    ui.add_space(6.0);
    if ui.button("Reset accumulation").clicked()
    {
        action = Some(ControlsAction::Reset);
    }
    action
}
