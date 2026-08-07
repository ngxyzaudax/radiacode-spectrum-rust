use egui::{CollapsingHeader, Color32, CornerRadius, RichText, Sense, Stroke, StrokeKind, Ui, Vec2};

use crate::theme::{ACCENT, MUTED};

pub fn settings_section(
    ui: &mut Ui,
    title: &str,
    hint: &str,
    add_contents: impl FnOnce(&mut Ui),
) {
    CollapsingHeader::new(RichText::new(title).strong())
        .default_open(true)
        .show(ui, |ui| {
            if !hint.is_empty() {
                ui.label(RichText::new(hint).small().color(MUTED));
                ui.add_space(4.0);
            }
            add_contents(ui);
        });
    ui.add_space(4.0);
}

pub fn toggle_knob(ui: &mut Ui, on: &mut bool) -> bool {
    let size = Vec2::new(32.0, 18.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let mut changed = false;
    if response.clicked() {
        *on = !*on;
        changed = true;
    }
    let how_on = ui.ctx().animate_bool_responsive(response.id, *on);
    let visuals = ui.style().interact_selectable(&response, *on);
    let track = if *on {
        ACCENT.gamma_multiply(0.85)
    } else {
        visuals.bg_fill
    };
    ui.painter()
        .rect(rect, CornerRadius::same(9), track, Stroke::NONE, StrokeKind::Inside);
    let radius = 0.5 * rect.height() - 2.0;
    let circle_x = egui::lerp((rect.left() + radius + 2.0)..=(rect.right() - radius - 2.0), how_on);
    ui.painter().circle(
        egui::pos2(circle_x, rect.center().y),
        radius,
        Color32::WHITE,
        Stroke::NONE,
    );
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    changed
}

pub fn toggle_switch(ui: &mut Ui, on: &mut bool, label: &str) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        changed |= toggle_knob(ui, on);
        ui.label(RichText::new(label).color(if *on {
            ui.visuals().text_color()
        } else {
            MUTED
        }));
    });
    changed
}
