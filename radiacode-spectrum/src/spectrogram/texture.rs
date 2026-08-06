use egui::ColorImage;
use tracing::debug;

use crate::spectrogram::colormap::normalized_to_color;
use crate::spectrogram::gap::display_count;
use crate::spectrogram::model::{SpectrogramRow, SpectrogramSeries};
use crate::spectrogram::settings::SpectrogramSettings;
use crate::spectrogram::zscale::{map_count, ZScaleRange};

pub struct SpectrogramTexture {
    pub image: ColorImage,
    pub dirty: bool,
}

impl SpectrogramTexture {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            image: ColorImage::filled(
                [width.max(1), height.max(1)],
                egui::Color32::from_rgb(8, 10, 16),
            ),
            dirty: true,
        }
    }

    pub fn rebuild(
        &mut self,
        series: &SpectrogramSeries,
        visible: &[SpectrogramRow],
        source_cols: &[usize],
        display_rows: usize,
        settings: &SpectrogramSettings,
        z_range: &ZScaleRange,
    ) {
        let width = source_cols.len().max(1);
        let height = display_rows.max(1);
        if self.image.width() != width || self.image.height() != height {
            self.image =
                ColorImage::filled([width, height], egui::Color32::from_rgb(8, 10, 16));
        } else {
            for pixel in &mut self.image.pixels {
                *pixel = egui::Color32::from_rgb(8, 10, 16);
            }
        }
        if source_cols.is_empty() || visible.is_empty() {
            self.dirty = true;
            return;
        }

        let capture_interval = series.header.interval_secs;
        let native = native_rows(visible, source_cols, capture_interval);
        let start_row = height.saturating_sub(native.len());
        let mut lit_pixels = 0usize;
        for (index, row) in native.iter().enumerate() {
            let texture_row = start_row + index;
            for (col, &count) in row.iter().enumerate() {
                let t = map_count(count as f32, &z_range);
                let color = if count == 0 {
                    egui::Color32::from_rgb(8, 10, 16)
                } else {
                    normalized_to_color(t, settings.palette)
                };
                if color != egui::Color32::from_rgb(8, 10, 16) {
                    lit_pixels += 1;
                }
                self.image[(col, texture_row)] = color;
            }
        }
        debug!(
            width,
            height,
            rows = native.len(),
            source_cols = source_cols.len(),
            lit_pixels,
            z_max = z_range.max,
            "spectrogram texture rebuilt"
        );
        self.dirty = true;
    }
}

fn native_rows(
    rows: &[SpectrogramRow],
    source_cols: &[usize],
    capture_interval_secs: f64,
) -> Vec<Vec<u32>> {
    rows.iter()
        .map(|row| {
            source_cols
                .iter()
                .map(|&index| {
                    let raw = row.counts.get(index).copied().unwrap_or(0);
                    display_count(
                        raw,
                        row.kind,
                        capture_interval_secs,
                        row.interval_secs,
                    )
                })
                .collect()
        })
        .collect()
}

pub fn source_columns(
    series: &SpectrogramSeries,
    energy_min_kev: f64,
    energy_max_kev: f64,
    channel_start: usize,
    display_cols: usize,
) -> Vec<usize> {
    let in_range: Vec<usize> = series
        .energies_kev
        .iter()
        .enumerate()
        .filter(|(_, energy)| (**energy >= energy_min_kev) && (**energy <= energy_max_kev))
        .map(|(index, _)| index)
        .collect();
    if in_range.is_empty() {
        return Vec::new();
    }
    let start = channel_start.min(in_range.len().saturating_sub(1));
    let end = (start + display_cols).min(in_range.len());
    in_range[start..end].to_vec()
}

#[cfg(test)]
mod tests {
    use super::{native_rows, source_columns, SpectrogramTexture};
    use crate::spectrogram::color_scheme::ColorScheme;
    use crate::spectrogram::gap::display_count;
    use crate::spectrogram::model::{RowKind, SpectrogramHeader, SpectrogramRow, SpectrogramSeries};
    use crate::spectrogram::settings::SpectrogramSettings;
    use crate::spectrogram::zscale::compute_series_z_range;

    #[test]
    fn native_row_maps_one_to_one() {
        let row = SpectrogramRow {
            elapsed_secs: 0.0,
            interval_secs: 5.0,
            kind: RowKind::Normal,
            counts: vec![1, 50, 2, 3],
        };
        let cols = vec![0, 1, 2, 3];
        assert_eq!(native_rows(&[row], &cols, 5.0)[0], vec![1, 50, 2, 3]);
    }

    #[test]
    fn gap_row_brightness_is_rate_normalized() {
        let raw = 1000;
        let scaled = display_count(
            raw,
            RowKind::GapRecovery {
                offline_secs: 50.0,
                raw_total: 1000,
            },
            5.0,
            50.0,
        );
        let row = SpectrogramRow {
            elapsed_secs: 0.0,
            interval_secs: 50.0,
            kind: RowKind::GapRecovery {
                offline_secs: 50.0,
                raw_total: 1000,
            },
            counts: vec![raw, 0, 0, 0],
        };
        let cols = vec![0, 1, 2, 3];
        assert_eq!(native_rows(&[row], &cols, 5.0)[0][0], scaled);
        assert!(scaled < raw);
    }

    #[test]
    fn source_columns_respects_window() {
        let header = SpectrogramHeader {
            created_at: "t".into(),
            a0: 0.0,
            a1: 1.0,
            a2: 0.0,
            channel_count: 4,
            interval_secs: 5.0,
            device_serial: None,
            energies_kev: vec![10.0, 20.0, 30.0, 40.0],
        };
        let series = SpectrogramSeries::new(header, vec![10.0, 20.0, 30.0, 40.0]);
        assert_eq!(source_columns(&series, 0.0, 3000.0, 1, 2), vec![1, 2]);
    }

    #[test]
    fn rebuild_lights_nonzero_bins() {
        let header = SpectrogramHeader {
            created_at: "t".into(),
            a0: 0.0,
            a1: 1.0,
            a2: 0.0,
            channel_count: 4,
            interval_secs: 5.0,
            device_serial: None,
            energies_kev: vec![10.0, 20.0, 30.0, 40.0],
        };
        let mut series = SpectrogramSeries::new(header, vec![10.0, 20.0, 30.0, 40.0]);
        series.push_row(vec![0, 50, 0, 10], 5.0, RowKind::Normal, 1000);
        let cols = vec![0, 1, 2, 3];
        let mut texture = SpectrogramTexture::new(1, 1);
        let z_range = compute_series_z_range(&series, &SpectrogramSettings::default());
        texture.rebuild(
            &series,
            &series.rows,
            &cols,
            8,
            &SpectrogramSettings {
                palette: ColorScheme::Viridis,
                ..SpectrogramSettings::default()
            },
            &z_range,
        );
        let lit = texture
            .image
            .pixels
            .iter()
            .filter(|pixel| **pixel != egui::Color32::from_rgb(8, 10, 16))
            .count();
        assert!(lit > 0);
    }
}
