use egui::{Align2, FontId, Rect};

use crate::theme::MUTED;
use crate::spectrogram::layout::SpectrogramLayout;
use crate::spectrogram::model::SpectrogramRow;

const MIN_LABEL_SPACING_PX: f32 = 14.0;

pub fn draw_time_axis(
    painter: &egui::Painter,
    image_rect: Rect,
    layout: SpectrogramLayout,
    visible: &[SpectrogramRow],
) {
    if visible.is_empty() {
        return;
    }
    let font = FontId::new(11.0, egui::FontFamily::Proportional);
    let stride = label_stride(layout.cell_px);
    let filled = visible.len();
    let row_offset = layout.display_rows.saturating_sub(filled);
    for local in 0..filled {
        if !should_label_row(local, filled, stride) {
            continue;
        }
        let y = image_rect.top()
            + (row_offset + local) as f32 * layout.cell_px
            + layout.cell_px * 0.5;
        painter.text(
            egui::pos2(image_rect.left() - 6.0, y),
            Align2::RIGHT_CENTER,
            format_duration(visible[local].elapsed_secs),
            font.clone(),
            MUTED,
        );
    }
}

fn label_stride(cell_px: f32) -> usize {
    ((MIN_LABEL_SPACING_PX / cell_px.max(0.001)).ceil() as usize).max(1)
}

fn should_label_row(local: usize, filled: usize, stride: usize) -> bool {
    let from_bottom = filled - 1 - local;
    from_bottom % stride == 0
}

fn format_duration(secs: f64) -> String {
    let total = secs.max(0.0).round() as u64;
    let hours = total / 3600;
    let minutes = (total % 3600) / 60;
    let seconds = total % 60;
    format!("{hours:02}:{minutes:02}:{seconds:02}")
}

#[cfg(test)]
mod tests {
    use super::{label_stride, should_label_row};

    #[test]
    fn dense_cells_label_every_row() {
        assert_eq!(label_stride(20.0), 1);
        assert!(should_label_row(0, 3, 1));
        assert!(should_label_row(1, 3, 1));
        assert!(should_label_row(2, 3, 1));
    }

    #[test]
    fn sparse_cells_skip_labels_but_keep_newest() {
        assert_eq!(label_stride(5.0), 3);
        assert!(!should_label_row(0, 5, 3));
        assert!(should_label_row(1, 5, 3));
        assert!(!should_label_row(2, 5, 3));
        assert!(!should_label_row(3, 5, 3));
        assert!(should_label_row(4, 5, 3));
    }
}
