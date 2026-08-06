use crate::spectrogram::colormap::percentile_peak;
use crate::spectrogram::gap::display_count;
use crate::spectrogram::model::{SpectrogramRow, SpectrogramSeries};
use crate::spectrogram::settings::SpectrogramSettings;

const FULL_ZSCAN_CELL_LIMIT: usize = 250_000;

#[derive(Clone, Copy, Debug)]
pub struct ZScaleRange {
    pub min: f32,
    pub max: f32,
}

pub fn compute_series_z_range(
    series: &SpectrogramSeries,
    settings: &SpectrogramSettings,
) -> ZScaleRange {
    if !settings.auto_brightness {
        return manual_z_range(settings);
    }
    resolve_z_range(settings, &series_display_values(series))
}

pub fn resolve_z_range(settings: &SpectrogramSettings, values: &[u32]) -> ZScaleRange {
    if settings.auto_brightness && !values.is_empty() {
        let peak = percentile_peak(values, 0.98).max(1.0);
        return ZScaleRange {
            min: 0.0,
            max: peak,
        };
    }
    manual_z_range(settings)
}

fn manual_z_range(settings: &SpectrogramSettings) -> ZScaleRange {
    ZScaleRange {
        min: settings.z_min,
        max: settings.z_max.max(settings.z_min + 1.0),
    }
}

fn series_display_values(series: &SpectrogramSeries) -> Vec<u32> {
    let capture_interval = series.header.interval_secs;
    let cell_count = series.rows.len().saturating_mul(series.energies_kev.len().max(1));
    if cell_count > FULL_ZSCAN_CELL_LIMIT {
        return row_peak_values(&series.rows, capture_interval);
    }
    series
        .rows
        .iter()
        .flat_map(|row| row_display_values(row, capture_interval))
        .filter(|&value| value > 0)
        .collect()
}

fn row_peak_values(rows: &[SpectrogramRow], target_interval_secs: f64) -> Vec<u32> {
    rows.iter()
        .filter_map(|row| {
            row.counts
                .iter()
                .map(|&raw| {
                    display_count(raw, row.kind, target_interval_secs, row.interval_secs)
                })
                .max()
        })
        .filter(|&value| value > 0)
        .collect()
}

fn row_display_values(
    row: &SpectrogramRow,
    target_interval_secs: f64,
) -> impl Iterator<Item = u32> + '_ {
    row.counts.iter().map(move |&raw| {
        display_count(raw, row.kind, target_interval_secs, row.interval_secs)
    })
}

pub fn map_count(value: f32, range: &ZScaleRange) -> f32 {
    if value <= 0.0 {
        return 0.0;
    }
    let span = (range.max - range.min).max(1.0);
    ((value - range.min) / span).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::{compute_series_z_range, map_count, resolve_z_range, ZScaleRange};
    use crate::spectrogram::model::{RowKind, SpectrogramHeader, SpectrogramSeries};
    use crate::spectrogram::settings::SpectrogramSettings;

    fn sample_series(counts: Vec<u32>) -> SpectrogramSeries {
        let header = SpectrogramHeader {
            created_at: "t".into(),
            a0: 0.0,
            a1: 1.0,
            a2: 0.0,
            channel_count: counts.len() as u32,
            interval_secs: 5.0,
            device_serial: None,
            energies_kev: (0..counts.len()).map(|index| index as f64).collect(),
        };
        let mut series = SpectrogramSeries::new(header, (0..counts.len()).map(|index| index as f64).collect());
        series.push_row(counts, 5.0, RowKind::Normal, 1000);
        series
    }

    #[test]
    fn global_range_ignores_missing_hot_channel_in_viewport() {
        let series = sample_series(vec![1, 100]);
        let global = compute_series_z_range(&series, &SpectrogramSettings::default());
        let local = resolve_z_range(&SpectrogramSettings::default(), &[1]);
        let global_t = map_count(1.0, &global);
        let local_t = map_count(1.0, &local);
        assert!(global_t < local_t);
    }

    #[test]
    fn auto_brightness_uses_peak() {
        let mut settings = SpectrogramSettings::default();
        settings.auto_brightness = true;
        let range = resolve_z_range(&settings, &[1, 2, 100]);
        assert!(range.max >= 100.0);
    }

    #[test]
    fn linear_mapping_is_monotonic() {
        let range = ZScaleRange {
            min: 0.0,
            max: 1000.0,
        };
        let low = map_count(10.0, &range);
        let high = map_count(100.0, &range);
        assert!(high > low);
    }
}
