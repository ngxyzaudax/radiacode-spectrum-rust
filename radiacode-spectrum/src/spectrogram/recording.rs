use std::path::PathBuf;

use tracing::info;

use crate::energy::energy_grid;
use crate::model::SpectrumView;
use crate::spectrogram::model::SpectrogramDisplay;
use crate::spectrogram::recording_seed::{
    ensure_live_series, recording_header, seed_writer_from_live,
};
use crate::spectrogram::state::SpectrogramState;
use crate::spectrogram::storage::{
    ensure_dir, load_recording, open_recording_append, timestamp_filename, RecordingWriter,
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
    let dir = ensure_dir(&cap.settings.recordings_dir).map_err(|error| error.to_string())?;
    let path = dir.join(timestamp_filename());
    let header = recording_header(&cap, spectrum, device_serial, grid.indices.len() as u32);
    info!(path = %path.display(), "spectrogram recording started");
    let mut writer =
        RecordingWriter::create(path, &header).map_err(|error| error.to_string())?;
    let seeded_rows = seed_writer_from_live(&mut writer, cap.live_series.as_ref())
        .map_err(|error| error.to_string())?;
    let continue_live = seeded_rows > 0 && cap.baseline.is_some();
    if !continue_live {
        cap.skip_next_sample = true;
    }
    cap.recording = Some(writer);
    cap.paused_recording_path = None;
    cap.capture_enabled = true;
    cap.last_auto_save = None;
    cap.status = if seeded_rows > 0 {
        format!("Recording started with {seeded_rows} existing row(s).")
    } else {
        "Recording started.".into()
    };
    cap.mark_dirty();
    Ok(())
}

pub fn pause_capture(state: &mut SpectrogramState) -> Result<(), String> {
    let mut cap = state
        .capture
        .lock()
        .map_err(|_| "capture lock failed".to_string())?;
    cap.capture_enabled = false;
    cap.status = if cap.recording.is_some() {
        "Recording paused.".into()
    } else {
        "Live capture paused.".into()
    };
    cap.mark_dirty();
    Ok(())
}

pub fn resume_capture(state: &mut SpectrogramState) -> Result<(), String> {
    let mut cap = state
        .capture
        .lock()
        .map_err(|_| "capture lock failed".to_string())?;
    cap.capture_enabled = true;
    cap.skip_next_sample = true;
    cap.status = if cap.recording.is_some() {
        "Recording.".into()
    } else {
        "Live capture.".into()
    };
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
    cap.capture_enabled = true;
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
            state.loaded_path = Some(path.clone());
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
