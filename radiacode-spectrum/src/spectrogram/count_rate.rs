use egui::{Pos2, Rect, Stroke};

use crate::theme::ACCENT;
use crate::spectrogram::layout::SpectrogramLayout;
use crate::spectrogram::model::SpectrogramRow;

pub fn draw_count_rate_overlay(
    painter: &egui::Painter,
    image_rect: Rect,
    layout: SpectrogramLayout,
    visible: &[SpectrogramRow],
    enabled: bool,
) {
    if !enabled || visible.is_empty() {
        return;
    }
    let totals: Vec<f32> = visible
        .iter()
        .map(|row| row.counts.iter().map(|&value| value as f32).sum())
        .collect();
    let peak = totals.iter().copied().fold(0.0_f32, f32::max).max(1.0);
    let width = 28.0;
    let left = image_rect.left() - width - 4.0;
    let points: Vec<Pos2> = totals
        .iter()
        .enumerate()
        .map(|(index, total)| {
            let y = count_rate_row_y(image_rect, layout, totals.len(), index);
            let x = left + (total / peak) * width;
            Pos2::new(x, y)
        })
        .collect();
    if points.len() >= 2 {
        painter.add(egui::Shape::line(points, Stroke::new(1.5, ACCENT)));
    }
}

fn count_rate_row_y(
    image_rect: Rect,
    layout: SpectrogramLayout,
    filled_rows: usize,
    index: usize,
) -> f32 {
    let row_offset = layout.display_rows.saturating_sub(filled_rows);
    image_rect.top() + (row_offset + index) as f32 * layout.cell_px + layout.cell_px * 0.5
}

#[cfg(test)]
mod tests {
    use egui::{pos2, Rect};

    use super::count_rate_row_y;
    use crate::spectrogram::layout::SpectrogramLayout;

    fn layout(display_rows: usize, cell_px: f32) -> SpectrogramLayout {
        SpectrogramLayout {
            cell_px,
            display_cols: 10,
            display_rows,
            image_rect: Rect::from_min_max(pos2(40.0, 0.0), pos2(140.0, display_rows as f32 * cell_px)),
        }
    }

    #[test]
    fn sparse_rows_sit_at_bottom() {
        let layout = layout(10, 10.0);
        let image = layout.image_rect;
        let y0 = count_rate_row_y(image, layout, 2, 0);
        let y1 = count_rate_row_y(image, layout, 2, 1);
        assert!((y0 - 85.0).abs() < 0.01);
        assert!((y1 - 95.0).abs() < 0.01);
    }

    #[test]
    fn full_viewport_uses_all_rows() {
        let layout = layout(4, 10.0);
        let image = layout.image_rect;
        let y0 = count_rate_row_y(image, layout, 4, 0);
        let y3 = count_rate_row_y(image, layout, 4, 3);
        assert!((y0 - 5.0).abs() < 0.01);
        assert!((y3 - 35.0).abs() < 0.01);
    }
}
