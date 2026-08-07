use egui::{Color32, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};

use crate::theme::{ACCENT, MUTED};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalIconKind {
    Sound,
    Vibro,
}

pub fn paint_signal_icon(ui: &mut Ui, kind: SignalIconKind, active: bool) {
    let size = Vec2::new(14.0, 14.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let color = if active { ACCENT } else { MUTED };
    match kind {
        SignalIconKind::Sound => paint_sound_icon(ui, rect, color),
        SignalIconKind::Vibro => paint_vibro_icon(ui, rect, color),
    }
}

fn paint_sound_icon(ui: &mut Ui, rect: Rect, color: Color32) {
    let painter = ui.painter();
    let cx = rect.center().x - 1.0;
    let cy = rect.center().y;
    let body = Rect::from_center_size(Pos2::new(cx - 2.0, cy), Vec2::new(4.0, 5.0));
    painter.rect_filled(body, 1.0, color);
    painter.add(egui::Shape::convex_polygon(
        vec![
            Pos2::new(cx, cy - 4.0),
            Pos2::new(cx + 4.0, cy - 6.0),
            Pos2::new(cx + 4.0, cy + 6.0),
            Pos2::new(cx, cy + 4.0),
        ],
        color,
        Stroke::NONE,
    ));
}

fn paint_vibro_icon(ui: &mut Ui, rect: Rect, color: Color32) {
    let painter = ui.painter();
    let cx = rect.center().x;
    let cy = rect.center().y;
    let phone = Rect::from_center_size(Pos2::new(cx, cy), Vec2::new(5.0, 9.0));
    painter.rect_stroke(phone, 1.0, Stroke::new(1.2, color), StrokeKind::Outside);
    for sign in [-1.0_f32, 1.0] {
        let x = cx + sign * 5.5;
        painter.line_segment(
            [Pos2::new(x, cy - 3.0), Pos2::new(x + sign * 1.5, cy)],
            Stroke::new(1.2, color),
        );
        painter.line_segment(
            [Pos2::new(x + sign * 1.5, cy), Pos2::new(x, cy + 3.0)],
            Stroke::new(1.2, color),
        );
    }
}
