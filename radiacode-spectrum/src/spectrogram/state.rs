use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::sync::atomic::Ordering;
use std::time::Instant;

use crate::model::SpectrumView;
use crate::spectrogram::baseline::IngestBaseline;
use crate::spectrogram::capture::SpectrogramCapture;
use crate::spectrogram::model::{RecordingEntry, SpectrogramDisplay, SpectrogramSeries};
use crate::spectrogram::library_meta::load_meta;
use crate::spectrogram::recording;
use crate::spectrogram::settings::{load_settings, save_settings, SpectrogramSettings};
use crate::spectrogram::storage::list_recordings;
use crate::spectrogram::texture::SpectrogramTexture;
use crate::spectrogram::view_range::SpectrogramViewRange;
use crate::spectrogram::zscale::{compute_series_z_range, ZScaleRange};

pub struct SpectrogramState {
    pub capture: Arc<Mutex<SpectrogramCapture>>,
    pub display: SpectrogramDisplay,
    pub live_series: Option<SpectrogramSeries>,
    pub loaded_series: Option<SpectrogramSeries>,
    pub loaded_path: Option<PathBuf>,
    pub paused_recording_path: Option<PathBuf>,
    pub history: Vec<RecordingEntry>,
    pub library_filter: String,
    pub library_edit_path: Option<PathBuf>,
    pub library_edit_name: String,
    pub library_edit_comment: String,
    pub texture: SpectrogramTexture,
    pub texture_handle: Option<egui::TextureHandle>,
    pub status: String,
    pub settings: SpectrogramSettings,
    pub view_range: SpectrogramViewRange,
    pub last_ingested_sequence: u64,
    pub skip_next_sample: bool,
    pub reconnect_baseline_pending: bool,
    pub last_ingest_at: Option<Instant>,
    pub last_auto_save: Option<Instant>,
    pub show_grid: bool,
    pub show_count_rate: bool,
    pub show_isotopes: bool,
    pub capture_enabled: bool,
    pub z_range: Option<ZScaleRange>,
    pub z_range_rows: usize,
    pub(crate) baseline: Option<IngestBaseline>,
}

impl SpectrogramState {
    pub fn new(capture: Arc<Mutex<SpectrogramCapture>>) -> Self {
        let settings = load_settings();
        if let Ok(mut cap) = capture.lock() {
            cap.settings = settings.clone();
        }
        Self {
            capture,
            display: SpectrogramDisplay::Live,
            live_series: None,
            loaded_series: None,
            loaded_path: None,
            paused_recording_path: None,
            history: Vec::new(),
            library_filter: String::new(),
            library_edit_path: None,
            library_edit_name: String::new(),
            library_edit_comment: String::new(),
            texture: SpectrogramTexture::new(1, 1),
            texture_handle: None,
            status: String::new(),
            settings: load_settings(),
            view_range: SpectrogramViewRange::new(),
            last_ingested_sequence: 0,
            skip_next_sample: false,
            reconnect_baseline_pending: false,
            last_ingest_at: None,
            last_auto_save: None,
            show_grid: true,
            show_count_rate: false,
            show_isotopes: false,
            capture_enabled: false,
            z_range: None,
            z_range_rows: 0,
            baseline: None,
        }
    }

    pub fn sync_from_capture(&mut self) {
        let Ok(cap) = self.capture.lock() else {
            return;
        };
        if !cap.dirty.load(Ordering::Acquire) {
            return;
        }
        self.baseline = cap.baseline.clone();
        let had_series = self.live_series.is_some();
        self.live_series = cap.live_series.clone();
        if let Some(series) = self.live_series.as_ref() {
            if had_series {
                self.view_range
                    .set_series_energy_bounds(&series.energies_kev);
            } else {
                self.view_range.fit_series_energy(&series.energies_kev);
            }
        }
        self.paused_recording_path = cap.paused_recording_path.clone();
        self.status = cap.status.clone();
        self.last_ingested_sequence = cap.last_ingested_sequence;
        self.skip_next_sample = cap.skip_next_sample;
        self.reconnect_baseline_pending = cap.reconnect_baseline_pending;
        self.last_ingest_at = cap.last_ingest_at;
        self.last_auto_save = cap.last_auto_save;
        self.capture_enabled = cap.capture_enabled;
        if self.live_series.is_some() {
            self.texture.dirty = true;
        }
        cap.dirty.store(false, Ordering::Release);
    }

    pub fn is_recording(&self) -> bool {
        self.capture
            .lock()
            .ok()
            .is_some_and(|cap| cap.recording.is_some())
    }

    pub fn on_reconnect(&mut self) {
        if let Ok(mut cap) = self.capture.lock() {
            cap.on_reconnect();
        }
        self.sync_from_capture();
    }

    pub fn on_tab_enter(&mut self) {
        if self.live_series.is_some() {
            self.texture.dirty = true;
            self.status = format!("Capturing {} spectrogram row(s).", self.live_row_count());
            return;
        }
        self.skip_next_sample = true;
        self.status = "Waiting for first fresh spectrum sample.".into();
    }

    pub fn close_loaded(&mut self) {
        self.loaded_series = None;
        self.loaded_path = None;
        self.display = SpectrogramDisplay::Live;
        self.z_range_rows = 0;
        self.texture.dirty = true;
        self.status = if self.live_series.is_some() {
            format!("Live spectrogram ({} rows).", self.live_row_count())
        } else {
            "Returned to live view.".into()
        };
    }

    pub fn active_series(&self) -> Option<&SpectrogramSeries> {
        match self.display {
            SpectrogramDisplay::Live => self.live_series.as_ref(),
            SpectrogramDisplay::Loaded => self.loaded_series.as_ref(),
        }
    }

    pub fn live_row_count(&self) -> usize {
        self.live_series.as_ref().map(|series| series.row_count()).unwrap_or(0)
    }

    pub fn refresh_history(&mut self) {
        self.history = list_recordings(&self.settings.recordings_dir).unwrap_or_default();
    }

    pub fn filtered_history(&self) -> Vec<RecordingEntry> {
        let needle = self.library_filter.trim().to_lowercase();
        if needle.is_empty() {
            return self.history.clone();
        }
        self.history
            .iter()
            .filter(|entry| {
                entry.name.to_lowercase().contains(&needle)
                    || entry.comment.to_lowercase().contains(&needle)
                    || entry
                        .device_serial
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&needle)
            })
            .cloned()
            .collect()
    }

    pub fn persist_settings(&mut self) {
        self.settings.clamp();
        let _ = save_settings(&self.settings);
    }

    pub fn is_capture_paused(&self) -> bool {
        !self.capture_enabled
    }

    pub fn can_resume_append(&self) -> bool {
        !self.is_recording() && self.paused_recording_path.is_some()
    }

    pub fn pause_capture(&mut self) -> Result<(), String> {
        let result = recording::pause_capture(self);
        self.sync_from_capture();
        result
    }

    pub fn resume_capture(&mut self) -> Result<(), String> {
        let result = recording::resume_capture(self);
        self.sync_from_capture();
        result
    }

    pub fn start_recording(
        &mut self,
        spectrum: Option<&SpectrumView>,
        device_serial: Option<&str>,
    ) -> Result<(), String> {
        let result = recording::start_recording(self, spectrum, device_serial);
        self.sync_from_capture();
        result
    }

    pub fn stop_recording(&mut self) -> Result<(), String> {
        let result = recording::stop_recording(self);
        self.sync_from_capture();
        if result.is_ok() {
            if let Some(path) = self.paused_recording_path.clone() {
                self.open_library_editor(&path);
            }
        }
        result
    }

    pub fn open_library_editor(&mut self, path: &Path) {
        let fallback = path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("recording");
        let (name, comment) = self
            .history
            .iter()
            .find(|entry| entry.path == path)
            .map(|entry| (entry.name.clone(), entry.comment.clone()))
            .unwrap_or_else(|| {
                let meta = load_meta(path, fallback);
                (meta.name, meta.comment)
            });
        self.library_edit_path = Some(path.to_path_buf());
        self.library_edit_name = name;
        self.library_edit_comment = comment;
    }

    pub fn resume_recording(
        &mut self,
        spectrum: Option<&SpectrumView>,
        device_serial: Option<&str>,
    ) -> Result<(), String> {
        let result = recording::resume_recording(self, spectrum, device_serial);
        self.sync_from_capture();
        result
    }

    pub fn request_load(&mut self, path: PathBuf) {
        recording::request_load(self, path);
    }

    pub fn on_disconnect(&mut self) {
        let _ = self.stop_recording();
        if let Ok(mut cap) = self.capture.lock() {
            cap.on_disconnect();
        }
        self.live_series = None;
        self.display = SpectrogramDisplay::Live;
        self.loaded_series = None;
        self.loaded_path = None;
        self.last_ingested_sequence = 0;
        self.skip_next_sample = false;
        self.reconnect_baseline_pending = false;
        self.last_ingest_at = None;
        self.baseline = None;
        self.last_auto_save = None;
        self.capture_enabled = false;
        self.z_range = None;
        self.z_range_rows = 0;
        self.view_range.reset();
        self.mark_texture_dirty_empty();
    }

    pub fn on_settings_changed(&mut self) {
        self.settings.clamp();
        let previous_interval = self
            .capture
            .lock()
            .ok()
            .map(|cap| cap.settings.capture_interval_secs)
            .unwrap_or(self.settings.capture_interval_secs);
        let interval_changed =
            (previous_interval - self.settings.capture_interval_secs).abs() > 1e-9;
        self.persist_settings();
        if let Ok(mut cap) = self.capture.lock() {
            cap.settings = self.settings.clone();
        }
        if interval_changed {
            self.reset_accumulation();
            self.apply_capture_interval_to_live_header();
            if let Ok(mut cap) = self.capture.lock() {
                cap.status = format!(
                    "Capture interval set to {:.0}s. Accumulation reset.",
                    self.settings.capture_interval_secs
                );
                cap.mark_dirty();
            }
            self.sync_from_capture();
        }
        self.refresh_history();
        self.z_range_rows = 0;
        self.texture.dirty = true;
    }

    fn apply_capture_interval_to_live_header(&mut self) {
        let interval = self.settings.capture_interval_secs;
        if let Ok(mut cap) = self.capture.lock() {
            if let Some(series) = cap.live_series.as_mut() {
                series.header.interval_secs = interval;
            }
            cap.mark_dirty();
        }
        if let Some(series) = self.live_series.as_mut() {
            series.header.interval_secs = interval;
        }
    }

    pub fn refresh_z_range(&mut self) {
        let snapshot = self
            .active_series()
            .map(|series| (compute_series_z_range(series, &self.settings), series.row_count()));
        match snapshot {
            Some((range, rows)) => {
                self.z_range = Some(range);
                self.z_range_rows = rows;
            }
            None => {
                self.z_range = None;
                self.z_range_rows = 0;
            }
        }
    }

    pub fn ensure_z_range(&mut self) {
        let rows = self.active_series().map(|series| series.row_count()).unwrap_or(0);
        if self.z_range.is_none() || self.z_range_rows != rows {
            self.refresh_z_range();
        }
    }

    pub fn reset_accumulation(&mut self) {
        if self.display == SpectrogramDisplay::Loaded {
            self.close_loaded();
        }
        if let Ok(mut cap) = self.capture.lock() {
            if let Some(series) = cap.live_series.as_mut() {
                series.rows.clear();
            }
            cap.baseline = None;
            cap.skip_next_sample = true;
            cap.reconnect_baseline_pending = false;
            cap.last_ingest_at = None;
            cap.last_ingested_sequence = 0;
            cap.status = "Accumulation cleared. Waiting for next spectrum sample.".into();
            cap.mark_dirty();
        }
        self.sync_from_capture();
        if let Some(series) = self.live_series.as_ref() {
            self.view_range.fit_series_energy(&series.energies_kev);
        } else {
            self.view_range.reset();
        }
        self.z_range = None;
        self.z_range_rows = 0;
        self.mark_texture_dirty_empty();
    }

    pub fn reset_view(&mut self) {
        let energies = self
            .active_series()
            .map(|series| series.energies_kev.clone());
        if let Some(energies_kev) = energies {
            self.view_range.fit_series_energy(&energies_kev);
        } else {
            self.view_range.reset();
        }
        self.texture.dirty = true;
    }

    #[cfg(test)]
    pub fn ingest_spectrum(
        &mut self,
        spectrum: &SpectrumView,
        device_serial: Option<&str>,
        sequence: u64,
    ) {
        crate::spectrogram::ingest::ingest_spectrum(self, spectrum, device_serial, sequence);
    }

    fn mark_texture_dirty_empty(&mut self) {
        self.texture = SpectrogramTexture::new(1, 1);
        self.texture_handle = None;
        self.texture.dirty = true;
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use crate::model::SpectrumView;
    use crate::spectrogram::capture::SpectrogramCapture;
    use crate::spectrogram::state::SpectrogramState;

    fn test_state() -> SpectrogramState {
        let capture = Arc::new(Mutex::new(SpectrogramCapture::new()));
        let mut state = SpectrogramState::new(capture);
        if let Ok(mut cap) = state.capture.lock() {
            cap.on_session_connect("test");
        }
        state.sync_from_capture();
        state
    }

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
    fn ingest_uses_interval_delta_after_baseline() {
        let mut state = test_state();
        state.settings.capture_interval_secs = 10.0;
        if let Ok(mut cap) = state.capture.lock() {
            cap.settings.capture_interval_secs = 10.0;
        }
        state.ingest_spectrum(&sample_spectrum(10, 5), None, 1);
        assert_eq!(state.live_row_count(), 0);
        state.ingest_spectrum(&sample_spectrum(20, 15), None, 2);
        assert_eq!(state.live_row_count(), 1);
        assert_eq!(state.live_series.as_ref().unwrap().rows[0].counts[0], 10);
    }

    #[test]
    fn tab_reenter_keeps_history() {
        let mut state = test_state();
        state.settings.capture_interval_secs = 10.0;
        if let Ok(mut cap) = state.capture.lock() {
            cap.settings.capture_interval_secs = 10.0;
        }
        state.ingest_spectrum(&sample_spectrum(10, 5), None, 1);
        state.ingest_spectrum(&sample_spectrum(20, 15), None, 2);
        assert_eq!(state.live_row_count(), 1);
        state.on_tab_enter();
        assert_eq!(state.live_row_count(), 1);
        state.ingest_spectrum(&sample_spectrum(35, 25), None, 3);
        assert_eq!(state.live_row_count(), 2);
    }
}
