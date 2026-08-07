use std::path::PathBuf;

use tracing::info;

use crate::energy::energy_grid;
use crate::model::SpectrumView;
use crate::spectrogram::capture::SpectrogramCapture;
use crate::spectrogram::model::SpectrogramDisplay;
use crate::spectrogram::state::SpectrogramState;
use crate::spectrogram::storage::{
    ensure_dir, header_now, load_recording, open_recording_append, timestamp_filename,
    RecordingWriter,
};

pub fn start_recording(
    state: &mut SpectrogramState,
    spectrum: Option<&SpectrumView>,
    device_serial: Option<&str>,
) -> Result<(), String> {
    let mut cap = state.capture.lock().map_err(|_| "capture lock failed".to_string())?;
    if cap.recording.is_some() {
        return Ok(());
    }
    let Some(spectrum) = spectrum else {
        return Err("Connect a device before recording.".into());
    };
    let grid = energy_grid(spectrum);
    if grid.indices.is_empty() {
        return Err("No channels in energy range.".into());
    }
    ensure_live_series(&mut cap, spectrum, device_serial, &grid.energies_kev);
    cap.skip_next_sample = true;
    let dir = ensure_dir(&cap.settings.recordings_dir).map_err(|error| error.to_string())?;
    let path = dir.join(timestamp_filename());
    let header = header_from_spectrum(
        spectrum,
        device_serial,
        grid.indices.len() as u32,
        cap.settings.capture_interval(),
    );
    info!(path = %path.display(), "spectrogram recording started");
    let writer = RecordingWriter::create(path, &header).map_err(|error| error.to_string())?;
    cap.recording = Some(writer);
    cap.paused_recording_path = None;
    cap.last_auto_save = None;
    cap.status = "Recording started.".into();
    cap.mark_dirty();
    Ok(())
}

pub fn stop_recording(state: &mut SpectrogramState) -> Result<(), String> {
    let mut cap = state.capture.lock().map_err(|_| "capture lock failed".to_string())?;
    let Some(writer) = cap.recording.take() else {
        return Ok(());
    };
    let path = writer.finalize().map_err(|error| error.to_string())?;
    info!(path = %path.display(), "spectrogram recording saved");
    cap.paused_recording_path = Some(path.clone());
    cap.status = format!("Saved {}. Resume to append.", path.display());
    cap.mark_dirty();
    drop(cap);
    state.refresh_history();
    Ok(())
}

pub fn resume_recording(
    state: &mut SpectrogramState,
    spectrum: Option<&SpectrumView>,
    device_serial: Option<&str>,
) -> Result<(), String> {
    let mut cap = state.capture.lock().map_err(|_| "capture lock failed".to_string())?;
    if cap.recording.is_some() {
        return Ok(());
    }
    let Some(path) = cap.paused_recording_path.clone() else {
        drop(cap);
        return start_recording(state, spectrum, device_serial);
    };
    let Some(spectrum) = spectrum else {
        return Err("Connect a device before resuming.".into());
    };
    let grid = energy_grid(spectrum);
    ensure_live_series(&mut cap, spectrum, device_serial, &grid.energies_kev);
    cap.skip_next_sample = true;
    let writer = open_recording_append(path.clone()).map_err(|error| error.to_string())?;
    cap.recording = Some(writer);
    cap.last_auto_save = None;
    cap.status = format!("Recording resumed to {}.", path.display());
    cap.mark_dirty();
    Ok(())
}

pub fn request_load(state: &mut SpectrogramState, path: PathBuf) {
    load_into_state(state, path);
}

pub fn load_into_state(state: &mut SpectrogramState, path: PathBuf) {
    match load_recording(&path) {
        Ok(series) => {
            state.loaded_series = Some(series);
            state.display = SpectrogramDisplay::Loaded;
            if let Some(loaded) = state.loaded_series.as_ref() {
                state.view_range.fit_series_energy(&loaded.energies_kev);
            }
            state.status = if state.is_recording() {
                format!("Viewing library file {} (recording continues).", path.display())
            } else {
                format!("Loaded {}", path.display())
            };
            state.texture.dirty = true;
            state.z_range_rows = 0;
        }
        Err(error) => state.status = format!("Load failed: {error}"),
    }
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
    let header = header_from_spectrum(
        spectrum,
        device_serial,
        energies_kev.len() as u32,
        capture.settings.capture_interval(),
    );
    capture.live_series =
        Some(crate::spectrogram::model::SpectrogramSeries::new(header, energies_kev.to_vec()));
}

fn header_from_spectrum(
    spectrum: &SpectrumView,
    device_serial: Option<&str>,
    channel_count: u32,
    interval_secs: f64,
) -> crate::spectrogram::model::SpectrogramHeader {
    let grid = energy_grid(spectrum);
    header_now(
        spectrum.a0,
        spectrum.a1,
        spectrum.a2,
        channel_count,
        interval_secs,
        device_serial.map(str::to_string),
        grid.energies_kev,
    )
}
