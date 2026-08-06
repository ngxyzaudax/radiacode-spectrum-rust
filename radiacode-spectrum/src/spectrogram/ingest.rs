use std::time::{Duration, Instant};

use tracing::{debug, warn};

use crate::energy::{energy_grid, sample_indices};
use crate::model::SpectrumView;
use crate::spectrogram::baseline::IngestBaseline;
use crate::spectrogram::gap::{classify_row, ClassifiedRow};
use crate::spectrogram::library;
use crate::spectrogram::state::SpectrogramState;
use crate::view_tab::ViewTab;

pub fn ingest_spectrum(
    state: &mut SpectrogramState,
    spectrum: &SpectrumView,
    device_serial: Option<&str>,
    sequence: u64,
    active_tab: ViewTab,
) {
    if sequence == state.last_ingested_sequence {
        return;
    }
    if !state.should_capture(active_tab) {
        debug!(?active_tab, sequence, "spectrogram ingest skipped: not capturing");
        state.last_ingested_sequence = sequence;
        return;
    }
    let grid = energy_grid(spectrum);
    if grid.indices.is_empty() {
        warn!(sequence, "spectrogram ingest skipped: empty energy range");
        state.status = "No channels in selected energy range.".into();
        state.last_ingested_sequence = sequence;
        return;
    }
    state.ensure_live_series(spectrum, device_serial, &grid.energies_kev);
    let cumulative = sample_indices(&grid, &spectrum.counts);
    let device_duration_secs = spectrum.duration.as_secs_f64();
    if should_store_baseline_only(state) {
        store_baseline(state, sequence, cumulative, device_duration_secs);
        return;
    }
    let Some(baseline) = state.baseline.clone() else {
        store_baseline(state, sequence, cumulative, device_duration_secs);
        return;
    };
    let recent_totals = state
        .live_series
        .as_ref()
        .map(|series| series.recent_row_totals(5))
        .unwrap_or_default();
    let capture_interval = state.settings.capture_interval();
    let classified = classify_row(
        spectrum,
        &baseline,
        &cumulative,
        capture_interval,
        &recent_totals,
    );
    append_classified_row(
        state,
        sequence,
        cumulative,
        device_duration_secs,
        classified,
    );
}

fn should_store_baseline_only(state: &SpectrogramState) -> bool {
    state.skip_next_sample || state.reconnect_baseline_pending
}

fn store_baseline(
    state: &mut SpectrogramState,
    sequence: u64,
    cumulative: Vec<u32>,
    device_duration_secs: f64,
) {
    debug!(sequence, "spectrogram baseline sample stored");
    state.skip_next_sample = false;
    state.reconnect_baseline_pending = false;
    state.baseline = Some(IngestBaseline::new(cumulative, device_duration_secs));
    state.last_ingested_sequence = sequence;
    state.last_ingest_at = Some(Instant::now());
    state.status = "Synced. Adding rows on each spectrum refresh.".into();
}

fn append_classified_row(
    state: &mut SpectrogramState,
    sequence: u64,
    cumulative: Vec<u32>,
    device_duration_secs: f64,
    classified: ClassifiedRow,
) {
    let max_samples = state.settings.max_samples;
    let row_total: u64 = classified.counts.iter().map(|&value| value as u64).sum();
    if let Some(series) = state.live_series.as_mut() {
        series.push_row(
            classified.counts.clone(),
            classified.interval_secs,
            classified.kind,
            max_samples,
        );
        debug!(
            sequence,
            rows = series.row_count(),
            row_total,
            interval_secs = classified.interval_secs,
            ?classified.kind,
            "spectrogram row appended"
        );
    }
    if let Some(writer) = state.recording.as_mut() {
        if let Some(row) = state.live_series.as_ref().and_then(|series| series.rows.last()) {
            if let Err(error) = writer.append_row(row) {
                warn!(%error, "spectrogram recording write failed");
                state.status = format!("Recording write failed: {error}");
            }
        }
    }
    state.baseline = Some(IngestBaseline::new(cumulative, device_duration_secs));
    state.last_ingested_sequence = sequence;
    state.last_ingest_at = Some(Instant::now());
    state.texture.dirty = true;
    state.status = if let Some(series) = state.live_series.as_ref() {
        format!(
            "{} ({} row(s))",
            classified.status,
            series.row_count()
        )
    } else {
        classified.status
    };
}

pub fn maybe_auto_save(state: &mut SpectrogramState) {
    if state.recording.is_none() {
        return;
    }
    let due = state
        .last_auto_save
        .map(|t| t.elapsed() >= Duration::from_secs(60))
        .unwrap_or(true);
    if !due {
        return;
    }
    let Some(series) = state.live_series.as_ref() else {
        return;
    };
    match library::auto_save_snapshot(series, state.recording.as_ref()) {
        Ok(path) => {
            state.last_auto_save = Some(Instant::now());
            debug!(path = %path.display(), "spectrogram auto-saved");
        }
        Err(error) => warn!(%error, "spectrogram auto-save failed"),
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::model::SpectrumView;
    use crate::spectrogram::model::RowKind;
    use crate::spectrogram::state::SpectrogramState;
    use crate::view_tab::ViewTab;

    fn sample_spectrum(total: u32, duration_secs: u64) -> SpectrumView {
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
    fn reconnect_baseline_then_normal_row() {
        let mut state = SpectrogramState::new();
        state.settings.capture_interval_secs = 5.0;
        state.on_session_connect();
        state.ingest_spectrum(&sample_spectrum(10, 5), None, 1, ViewTab::Monitor);
        state.ingest_spectrum(&sample_spectrum(20, 10), None, 2, ViewTab::Monitor);
        assert_eq!(state.live_row_count(), 1);

        state.on_reconnect();
        state.ingest_spectrum(&sample_spectrum(5000, 60), None, 3, ViewTab::Monitor);
        assert_eq!(state.live_row_count(), 1);
        state.ingest_spectrum(&sample_spectrum(5010, 65), None, 4, ViewTab::Monitor);
        assert_eq!(state.live_row_count(), 2);
        assert!(matches!(
            state.live_series.as_ref().unwrap().rows[1].kind,
            RowKind::Normal
        ));
    }

    #[test]
    fn long_gap_produces_gap_recovery_row() {
        let mut state = SpectrogramState::new();
        state.settings.capture_interval_secs = 5.0;
        state.on_session_connect();
        state.ingest_spectrum(&sample_spectrum(10, 5), None, 1, ViewTab::Monitor);
        state.ingest_spectrum(&sample_spectrum(20, 10), None, 2, ViewTab::Monitor);
        state.ingest_spectrum(&sample_spectrum(2000, 55), None, 3, ViewTab::Monitor);
        assert_eq!(state.live_row_count(), 2);
        assert!(matches!(
            state.live_series.as_ref().unwrap().rows[1].kind,
            RowKind::GapRecovery { .. }
        ));
    }
}
