use std::path::PathBuf;

use tracing::info;

use crate::energy::energy_grid;
use crate::model::SpectrumView;
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
    if state.recording.is_some() {
        return Ok(());
    }
    let Some(spectrum) = spectrum else {
        return Err("Connect a device before recording.".into());
    };
    let grid = energy_grid(spectrum);
    if grid.indices.is_empty() {
        return Err("No channels in energy range.".into());
    }
    state.ensure_live_series(spectrum, device_serial, &grid.energies_kev);
    state.skip_next_sample = true;
    let dir = ensure_dir().map_err(|error| error.to_string())?;
    let path = dir.join(timestamp_filename());
    let header = header_from_spectrum(
        spectrum,
        device_serial,
        grid.indices.len() as u32,
        state.settings.capture_interval(),
    );
    info!(path = %path.display(), "spectrogram recording started");
    let writer = RecordingWriter::create(path, &header).map_err(|error| error.to_string())?;
    state.recording = Some(writer);
    state.paused_recording_path = None;
    state.last_auto_save = None;
    state.status = "Recording started.".into();
    Ok(())
}

pub fn stop_recording(state: &mut SpectrogramState) -> Result<(), String> {
    let Some(writer) = state.recording.take() else {
        return Ok(());
    };
    let path = writer.finalize().map_err(|error| error.to_string())?;
    info!(path = %path.display(), "spectrogram recording saved");
    state.paused_recording_path = Some(path.clone());
    state.status = format!("Saved {}. Resume to append.", path.display());
    state.refresh_history();
    Ok(())
}

pub fn resume_recording(
    state: &mut SpectrogramState,
    spectrum: Option<&SpectrumView>,
    device_serial: Option<&str>,
) -> Result<(), String> {
    if state.recording.is_some() {
        return Ok(());
    }
    let Some(path) = state.paused_recording_path.clone() else {
        return start_recording(state, spectrum, device_serial);
    };
    let Some(spectrum) = spectrum else {
        return Err("Connect a device before resuming.".into());
    };
    let grid = energy_grid(spectrum);
    state.ensure_live_series(spectrum, device_serial, &grid.energies_kev);
    state.skip_next_sample = true;
    let writer = open_recording_append(path.clone()).map_err(|error| error.to_string())?;
    state.recording = Some(writer);
    state.last_auto_save = None;
    state.status = format!("Recording resumed to {}.", path.display());
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
            state.status = if state.recording.is_some() {
                format!("Viewing library file {} (recording continues).", path.display())
            } else {
                format!("Loaded {}", path.display())
            };
            state.texture.dirty = true;
        }
        Err(error) => state.status = format!("Load failed: {error}"),
    }
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
