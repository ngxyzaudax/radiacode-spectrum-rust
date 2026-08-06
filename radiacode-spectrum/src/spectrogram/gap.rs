use crate::model::SpectrumView;
use crate::spectrogram::baseline::IngestBaseline;
use crate::spectrogram::delta::interval_row_counts;
use crate::spectrogram::model::RowKind;

const GAP_DEVICE_FACTOR: f64 = 1.5;
const GAP_WALL_MIN_SECS: f64 = 10.0;
const GAP_WALL_FACTOR: f64 = 3.0;
const SPIKE_MEDIAN_FACTOR: u64 = 15;
const SPIKE_MIN_COUNTS: u64 = 250;
const SPIKE_FLOOR: u64 = 200;
const MIN_ROW_DEVICE_FACTOR: f64 = 0.85;

pub struct ClassifiedRow {
    pub kind: RowKind,
    pub counts: Vec<u32>,
    pub interval_secs: f64,
    pub status: String,
}

pub fn classify_row(
    spectrum: &SpectrumView,
    baseline: &IngestBaseline,
    cumulative: &[u32],
    capture_interval_secs: f64,
    recent_row_totals: &[u64],
) -> ClassifiedRow {
    let row_counts = interval_row_counts(&baseline.counts, cumulative);
    let row_total = row_counts.iter().map(|&value| value as u64).sum();
    let device_duration_delta =
        spectrum.duration.as_secs_f64() - baseline.device_duration_secs;
    let wall_gap = baseline.ingested_at.elapsed().as_secs_f64();
    let wall_threshold = GAP_WALL_MIN_SECS.max(capture_interval_secs * GAP_WALL_FACTOR);
    if device_duration_delta > capture_interval_secs * GAP_DEVICE_FACTOR
        || wall_gap > wall_threshold
    {
        let offline_secs = device_duration_delta.max(wall_gap).max(capture_interval_secs);
        return ClassifiedRow {
            kind: RowKind::GapRecovery {
                offline_secs,
                raw_total: row_total,
            },
            counts: row_counts,
            interval_secs: offline_secs,
            status: format!(
                "Offline {:.0} s recovered — {row_total} counts (rate-normalized row)",
                offline_secs
            ),
        };
    }
    if is_live_spike(row_total, recent_row_totals) {
        let rate_factor = spike_rate_factor(row_total, recent_row_totals);
        return ClassifiedRow {
            kind: RowKind::LiveSpike { rate_factor },
            counts: row_counts,
            interval_secs: capture_interval_secs,
            status: format!(
                "Elevated interval — {row_total} counts ({rate_factor:.1}× recent median)"
            ),
        };
    }
    ClassifiedRow {
        kind: RowKind::Normal,
        counts: row_counts,
        interval_secs: effective_interval_secs(
            device_duration_delta,
            wall_gap,
            capture_interval_secs,
        ),
        status: format!("Capturing row with {row_total} counts."),
    }
}

pub fn device_timeline_regressed(
    device_duration_delta: f64,
    baseline_counts: &[u32],
    cumulative: &[u32],
) -> bool {
    if device_duration_delta < -1.0 {
        return true;
    }
    let baseline_total = channel_total(baseline_counts);
    let current_total = channel_total(cumulative);
    baseline_total > 64 && current_total + baseline_total / 20 < baseline_total
}

fn effective_interval_secs(
    device_duration_delta: f64,
    wall_gap: f64,
    capture_interval_secs: f64,
) -> f64 {
    let device_ready = row_interval_ready(device_duration_delta, capture_interval_secs);
    let interval = if device_ready {
        device_duration_delta
    } else {
        wall_gap
    };
    interval.max(0.1).min(capture_interval_secs * 2.0)
}

fn channel_total(values: &[u32]) -> u64 {
    values.iter().map(|&value| value as u64).sum()
}

pub fn row_interval_ready(
    device_duration_delta: f64,
    capture_interval_secs: f64,
) -> bool {
    device_duration_delta >= capture_interval_secs * MIN_ROW_DEVICE_FACTOR
}

pub fn display_count(
    raw: u32,
    kind: RowKind,
    target_interval_secs: f64,
    row_interval_secs: f64,
) -> u32 {
    if raw == 0 {
        return 0;
    }
    match kind {
        RowKind::GapRecovery { offline_secs, .. } => {
            if offline_secs <= 0.0 {
                return raw;
            }
            let scale = target_interval_secs / offline_secs;
            ((raw as f64) * scale).round().max(0.0) as u32
        }
        RowKind::LiveSpike { .. } => raw,
        RowKind::Normal => scale_to_target_interval(raw, row_interval_secs, target_interval_secs),
    }
}

fn scale_to_target_interval(raw: u32, row_interval_secs: f64, target_interval_secs: f64) -> u32 {
    if row_interval_secs <= 0.0 {
        return raw;
    }
    let scale = target_interval_secs / row_interval_secs;
    if (scale - 1.0).abs() < 0.05 {
        return raw;
    }
    ((raw as f64) * scale).round().max(0.0) as u32
}

fn is_live_spike(row_total: u64, recent_row_totals: &[u64]) -> bool {
    if row_total < SPIKE_MIN_COUNTS {
        return false;
    }
    if recent_row_totals.is_empty() {
        return row_total > SPIKE_FLOOR;
    }
    let mut sorted = recent_row_totals.to_vec();
    sorted.sort_unstable();
    let median = sorted[sorted.len() / 2];
    row_total > median.saturating_mul(SPIKE_MEDIAN_FACTOR).max(SPIKE_FLOOR)
}

fn spike_rate_factor(row_total: u64, recent_row_totals: &[u64]) -> f32 {
    if recent_row_totals.is_empty() {
        return 1.0;
    }
    let mut sorted = recent_row_totals.to_vec();
    sorted.sort_unstable();
    let median = sorted[sorted.len() / 2].max(1);
    (row_total as f32 / median as f32).max(1.0)
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use crate::model::SpectrumView;
    use crate::spectrogram::baseline::IngestBaseline;
    use crate::spectrogram::model::RowKind;

    use super::{classify_row, display_count};

    fn spectrum(total: u32, duration_secs: u64) -> SpectrumView {
        SpectrumView {
            duration: Duration::from_secs(duration_secs),
            a0: 0.0,
            a1: 1.0,
            a2: 0.0,
            counts: vec![total; 512],
            total_counts: total as u64 * 512,
        }
    }

    #[test]
    fn gap_row_when_device_duration_jumps() {
        let baseline = IngestBaseline {
            counts: vec![10; 512],
            device_duration_secs: 5.0,
            ingested_at: Instant::now(),
        };
        let cumulative = vec![5000; 512];
        let classified = classify_row(
            &spectrum(5000, 50),
            &baseline,
            &cumulative,
            5.0,
            &[20 * 512],
        );
        assert!(matches!(classified.kind, RowKind::GapRecovery { .. }));
        assert!((classified.interval_secs - 45.0).abs() < 0.1);
    }

    #[test]
    fn live_spike_when_connected_and_elevated() {
        let baseline = IngestBaseline {
            counts: vec![100; 512],
            device_duration_secs: 10.0,
            ingested_at: Instant::now(),
        };
        let cumulative = vec![5000; 512];
        let recent: Vec<u64> = (0..5).map(|_| 22 * 512).collect();
        let classified = classify_row(
            &spectrum(5000, 15),
            &baseline,
            &cumulative,
            5.0,
            &recent,
        );
        assert!(matches!(classified.kind, RowKind::LiveSpike { .. }));
    }

    #[test]
    fn device_timeline_regressed_on_duration_drop() {
        assert!(super::device_timeline_regressed(-10.0, &[100; 4], &[120; 4]));
    }

    #[test]
    fn device_timeline_regressed_on_count_drop() {
        assert!(super::device_timeline_regressed(5.0, &[1000; 4], &[10; 4]));
    }

    #[test]
    fn row_interval_ready_requires_capture_window() {
        assert!(super::row_interval_ready(8.5, 10.0));
        assert!(!super::row_interval_ready(4.0, 10.0));
    }

    #[test]
    fn normal_row_scales_partial_interval() {
        let scaled = display_count(100, RowKind::Normal, 10.0, 5.0);
        assert_eq!(scaled, 200);
    }

    #[test]
    fn gap_display_scales_brightness_down() {
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
        assert!(scaled < raw);
        assert_eq!(scaled, 100);
    }
}
