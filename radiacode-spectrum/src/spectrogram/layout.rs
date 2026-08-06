use egui::{pos2, Rect, Vec2};

pub const MIN_CELL_PX: f32 = 3.0;
pub const MIN_OVERVIEW_CELL_PX: f32 = 0.5;
pub const DEFAULT_EMPTY_CHANNELS: usize = 1024;

#[derive(Debug, Clone, Copy)]
pub struct SpectrogramLayout {
    pub cell_px: f32,
    pub display_cols: usize,
    pub display_rows: usize,
    pub image_rect: Rect,
}

pub fn compute_layout(
    plot_rect: Rect,
    channels_in_view: usize,
    fit_full_spectrum: bool,
) -> SpectrogramLayout {
    let plot = plot_rect.shrink(1.0);
    let channels = channels_in_view.max(1);
    let max_cols = ((plot.width() / MIN_CELL_PX).floor() as usize).max(1);
    let display_cols = if fit_full_spectrum {
        channels
    } else {
        channels.min(max_cols).max(1)
    };

    let cell_px = (plot.width() / display_cols as f32)
        .clamp(MIN_OVERVIEW_CELL_PX, plot.width().max(MIN_OVERVIEW_CELL_PX));
    let display_rows = ((plot.height() / cell_px).floor() as usize).max(1);

    let image_size = Vec2::new(
        (display_cols as f32 * cell_px).min(plot.width()),
        (display_rows as f32 * cell_px).min(plot.height()),
    );
    let left = plot.left();
    let bottom = plot.bottom();
    let top = (bottom - image_size.y).max(plot.top());
    let right = (left + image_size.x).min(plot.right());
    let image_rect = Rect::from_min_max(pos2(left, top), pos2(right, bottom));

    SpectrogramLayout {
        cell_px,
        display_cols,
        display_rows,
        image_rect,
    }
}

pub fn channels_in_energy_range(energies_kev: &[f64], energy_min: f64, energy_max: f64) -> usize {
    energies_kev
        .iter()
        .filter(|energy| **energy >= energy_min && **energy <= energy_max)
        .count()
}

#[cfg(test)]
mod tests {
    use egui::Rect;

    use super::compute_layout;

    #[test]
    fn cells_are_square() {
        let rect = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(400.0, 300.0));
        let layout = compute_layout(rect, 80, false);
        let expected = layout.image_rect.width() / layout.display_cols as f32;
        assert!((layout.cell_px - expected).abs() < 0.01);
        assert_eq!(
            layout.display_rows,
            (layout.image_rect.height() / layout.cell_px).floor() as usize
        );
    }

    #[test]
    fn viewport_rows_ignore_history_count() {
        let rect = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(400.0, 300.0));
        let empty = compute_layout(rect, 80, false);
        let full = compute_layout(rect, 80, false);
        assert_eq!(empty.display_rows, full.display_rows);
        assert!((empty.image_rect.width() - full.image_rect.width()).abs() < 1.0);
    }

    #[test]
    fn fit_full_keeps_full_width() {
        let rect = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(400.0, 300.0));
        let layout = compute_layout(rect, 1024, true);
        assert_eq!(layout.display_cols, 1024);
        assert!((layout.image_rect.width() - 398.0).abs() < 1.0);
    }

    #[test]
    fn few_channels_make_larger_squares() {
        let rect = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(400.0, 300.0));
        let wide = compute_layout(rect, 1024, true);
        let zoomed = compute_layout(rect, 40, false);
        assert!(zoomed.cell_px > wide.cell_px);
        assert!(zoomed.display_rows < wide.display_rows || zoomed.cell_px >= 3.0);
    }
}
