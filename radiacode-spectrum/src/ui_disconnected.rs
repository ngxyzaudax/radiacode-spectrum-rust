use egui::{RichText, Ui};

use crate::model::ConnectionState;
use crate::theme::MUTED;

pub fn draw_disconnected_view(ui: &mut Ui, connection: ConnectionState) {
    ui.vertical_centered(|ui| {
        ui.add_space(ui.available_height() * 0.35);
        let message = match connection {
            ConnectionState::Connecting => "Connecting to device…",
            ConnectionState::Disconnected | ConnectionState::Connected => {
                "Connect a device to start monitoring."
            }
        };
        ui.label(RichText::new(message).size(18.0).color(MUTED));
    });
}

pub fn shows_tab_content(connection: ConnectionState) -> bool {
    connection == ConnectionState::Connected
}

pub fn tab_works_offline(tab: crate::view_tab::ViewTab) -> bool {
    matches!(
        tab,
        crate::view_tab::ViewTab::Settings
            | crate::view_tab::ViewTab::Analysis
            | crate::view_tab::ViewTab::About
    )
}
