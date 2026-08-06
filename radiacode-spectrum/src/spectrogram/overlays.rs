use egui::{Color32, Pos2, Rect, Stroke};

use crate::theme::{ACCENT, MUTED};
use crate::spectrogram::layout::SpectrogramLayout;

pub struct IsotopeLine {
    pub name: &'static str,
    pub energy_kev: f64,
}

pub const DEFAULT_ISOTOPES: [IsotopeLine; 4] = [
    IsotopeLine {
        name: "K-40",
        energy_kev: 1460.8,
    },
    IsotopeLine {
        name: "Cs-137",
        energy_kev: 661.7,
    },
    IsotopeLine {
        name: "Co-60",
        energy_kev: 1173.2,
    },
    IsotopeLine {
        name: "Am-241",
        energy_kev: 59.5,
    },
];

pub fn draw_grid(
    painter: &egui::Painter,
    image_rect: Rect,
    layout: SpectrogramLayout,
    enabled: bool,
) {
    if !enabled || layout.cell_px <= 3.0 {
        return;
    }
    let stroke = Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 18));
    let cols = layout.display_cols as i32;
    let rows = layout.display_rows as i32;
    for col in 0..=cols {
        let x = image_rect.left() + col as f32 * layout.cell_px;
        painter.line_segment([Pos2::new(x, image_rect.top()), Pos2::new(x, image_rect.bottom())], stroke);
    }
    for row in 0..=rows {
        let y = image_rect.top() + row as f32 * layout.cell_px;
        painter.line_segment([Pos2::new(image_rect.left(), y), Pos2::new(image_rect.right(), y)], stroke);
    }
}

pub fn draw_crosshair(
    painter: &egui::Painter,
    hover: Pos2,
    image_rect: Rect,
) {
    if !image_rect.contains(hover) {
        return;
    }
    let stroke = Stroke::new(1.0, ACCENT);
    painter.line_segment(
        [Pos2::new(hover.x, image_rect.top()), Pos2::new(hover.x, image_rect.bottom())],
        stroke,
    );
    painter.line_segment(
        [Pos2::new(image_rect.left(), hover.y), Pos2::new(image_rect.right(), hover.y)],
        stroke,
    );
}

pub fn draw_isotope_lines(
    painter: &egui::Painter,
    image_rect: Rect,
    energy_min: f64,
    energy_max: f64,
    enabled: bool,
) {
    if !enabled {
        return;
    }
    let span = (energy_max - energy_min).max(1.0);
    let stroke = Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 220, 80, 160));
    for line in DEFAULT_ISOTOPES {
        if line.energy_kev < energy_min || line.energy_kev > energy_max {
            continue;
        }
        let t = ((line.energy_kev - energy_min) / span) as f32;
        let x = egui::lerp(image_rect.left()..=image_rect.right(), t);
        painter.line_segment(
            [Pos2::new(x, image_rect.top()), Pos2::new(x, image_rect.bottom())],
            stroke,
        );
        painter.text(
            Pos2::new(x + 2.0, image_rect.top() + 2.0),
            egui::Align2::LEFT_TOP,
            line.name,
            egui::FontId::new(10.0, egui::FontFamily::Proportional),
            MUTED,
        );
    }
}
