use std::time::{Duration, Instant};

use tracing::{debug, warn};

use crate::energy::{energy_grid, sample_indices};
use crate::model::SpectrumView;
use crate::spectrogram::baseline::IngestBaseline;
use crate::spectrogram::capture::SpectrogramCapture;
use crate::spectrogram::gap::{self, classify_row, ClassifiedRow};
use crate::spectrogram::library;
use crate::spectrogram::model::SpectrogramSeries;

pub fn ingest_capture(capture: &mut SpectrogramCapture, spectrum: &SpectrumView, sequence: u64) {
    if sequence <= capture.last_ingested_sequence {
        return;
    }
    if !capture.capture_enabled {
        capture.last_ingested_sequence = sequence;
        return;
    }
    let grid = energy_grid(spectrum);
    if grid.indices.is_empty() {
        warn!(sequence, "spectrogram ingest skipped: empty energy range");
        capture.status = "No channels in selected energy range.".into();
        capture.last_ingested_sequence = sequence;
        capture.mark_dirty();
        return;
    }
    let device_serial = capture.device_serial.clone();
    ensure_live_series(
        capture,
        spectrum,
        device_serial.as_deref(),
        &grid.energies_kev,
    );
    let cumulative = sample_indices(&grid, &spectrum.counts);
    let device_duration_secs = spectrum.duration.as_secs_f64();
    if capture.skip_next_sample || capture.reconnect_baseline_pending {
        store_baseline_capture(capture, sequence, cumulative, device_duration_secs);
        return;
    }
    let Some(baseline) = capture.baseline.clone() else {
        store_baseline_capture(capture, sequence, cumulative, device_duration_secs);
        return;
    };
    let recent_totals = capture
        .live_series
        .as_ref()
        .map(|series| series.recent_row_totals(5))
        .unwrap_or_default();
    let capture_interval = capture.settings.capture_interval();
    let device_duration_delta = device_duration_secs - baseline.device_duration_secs;
    let wall_gap = baseline.ingested_at.elapsed().as_secs_f64();
    if gap::device_timeline_regressed(device_duration_delta, &baseline.counts, &cumulative) {
        debug!(
            sequence,
            device_duration_delta,
            "spectrogram device timeline regressed, re-baselining"
        );
        store_baseline_capture(capture, sequence, cumulative, device_duration_secs);
        capture.status =
            "Device spectrum reset detected. Re-synced baseline; rows resume on next interval."
                .into();
        return;
    }
    if !gap::row_interval_ready(device_duration_delta, capture_interval)
        && !gap::row_interval_ready(wall_gap, capture_interval)
    {
        debug!(
            sequence,
            device_duration_delta,
            wall_gap,
            capture_interval,
            "spectrogram ingest skipped: interval not elapsed"
        );
        capture.last_ingested_sequence = sequence;
        return;
    }
    let classified = classify_row(
        spectrum,
        &baseline,
        &cumulative,
        capture_interval,
        &recent_totals,
    );
    append_classified_row_capture(
        capture,
        sequence,
        cumulative,
        device_duration_secs,
        classified,
    );
}

fn ensure_live_series(
    capture: &mut SpectrogramCapture,
    spectrum: &SpectrumView,
    device_serial: Option<&str>,
    energies_kev: &[f64],
) {
    if capture.live_series.is_some() {
        return;
    }
    let header = crate::spectrogram::storage::header_now(
        spectrum.a0,
        spectrum.a1,
        spectrum.a2,
        energies_kev.len() as u32,
        capture.settings.capture_interval(),
        device_serial.map(str::to_string),
        energies_kev.to_vec(),
    );
    capture.live_series = Some(SpectrogramSeries::new(header, energies_kev.to_vec()));
}

fn store_baseline_capture(
    capture: &mut SpectrogramCapture,
    sequence: u64,
    cumulative: Vec<u32>,
    device_duration_secs: f64,
) {
    debug!(sequence, "spectrogram baseline sample stored");
    capture.skip_next_sample = false;
    capture.reconnect_baseline_pending = false;
    capture.baseline = Some(IngestBaseline::new(cumulative, device_duration_secs));
    capture.last_ingested_sequence = sequence;
    capture.last_ingest_at = Some(Instant::now());
    capture.status = "Synced. Adding rows on each spectrum refresh.".into();
    capture.mark_dirty();
}

fn append_classified_row_capture(
    capture: &mut SpectrogramCapture,
    sequence: u64,
    cumulative: Vec<u32>,
    device_duration_secs: f64,
    classified: ClassifiedRow,
) {
    let max_samples = capture.settings.max_samples;
    let row_total: u64 = classified.counts.iter().map(|&value| value as u64).sum();
    if let Some(series) = capture.live_series.as_mut() {
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
    if let Some(writer) = capture.recording.as_mut() {
        if let Some(row) = capture.live_series.as_ref().and_then(|series| series.rows.last()) {
            if let Err(error) = writer.append_row(row) {
                warn!(%error, "spectrogram recording write failed");
                capture.status = format!("Recording write failed: {error}");
            }
        }
    }
    capture.baseline = Some(IngestBaseline::new(cumulative, device_duration_secs));
    capture.last_ingested_sequence = sequence;
    capture.last_ingest_at = Some(Instant::now());
    capture.status = if let Some(series) = capture.live_series.as_ref() {
        format!(
            "{} ({} row(s))",
            classified.status,
            series.row_count()
        )
    } else {
        classified.status
    };
    capture.mark_dirty();
}

pub fn maybe_auto_save_capture(capture: &mut SpectrogramCapture) {
    if capture.recording.is_none() {
        return;
    }
    let due = capture
        .last_auto_save
        .map(|t| t.elapsed() >= Duration::from_secs(60))
        .unwrap_or(true);
    if !due {
        return;
    }
    let Some(series) = capture.live_series.as_ref() else {
        return;
    };
    match library::auto_save_snapshot(
        series,
        capture.recording.as_ref(),
        &capture.settings.recordings_dir,
    ) {
        Ok(path) => {
            capture.last_auto_save = Some(Instant::now());
            debug!(path = %path.display(), "spectrogram auto-saved");
        }
        Err(error) => warn!(%error, "spectrogram auto-save failed"),
    }
}

#[cfg(test)]
pub fn ingest_spectrum(
    state: &mut crate::spectrogram::state::SpectrogramState,
    spectrum: &SpectrumView,
    device_serial: Option<&str>,
    sequence: u64,
) {
    if sequence == state.last_ingested_sequence {
        return;
    }
    if let Ok(mut capture) = state.capture.lock() {
        capture.device_serial = device_serial.map(str::to_string);
        ingest_capture(&mut capture, spectrum, sequence);
    }
    state.sync_from_capture();
}

#[cfg(test)]
#[path = "ingest_tests.rs"]
mod tests;
