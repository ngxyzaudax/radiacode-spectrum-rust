use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use crossbeam_channel::{Receiver, Sender};
use tracing::{debug, info};

use crate::model::SpectrumView;
use crate::spectrogram::baseline::IngestBaseline;
use crate::spectrogram::ingest;
use crate::spectrogram::model::SpectrogramSeries;
use crate::spectrogram::settings::{load_settings, SpectrogramSettings};
use crate::spectrogram::storage::RecordingWriter;

pub struct SpectrogramCapture {
    pub live_series: Option<SpectrogramSeries>,
    pub recording: Option<RecordingWriter>,
    pub paused_recording_path: Option<std::path::PathBuf>,
    pub settings: SpectrogramSettings,
    pub status: String,
    pub capture_enabled: bool,
    pub skip_next_sample: bool,
    pub reconnect_baseline_pending: bool,
    pub last_ingested_sequence: u64,
    pub last_ingest_at: Option<Instant>,
    pub last_auto_save: Option<Instant>,
    pub device_serial: Option<String>,
    pub dirty: AtomicBool,
    pub(crate) baseline: Option<IngestBaseline>,
}

impl SpectrogramCapture {
    pub fn new() -> Self {
        Self {
            live_series: None,
            recording: None,
            paused_recording_path: None,
            settings: load_settings(),
            status: String::new(),
            capture_enabled: false,
            skip_next_sample: false,
            reconnect_baseline_pending: false,
            last_ingested_sequence: 0,
            last_ingest_at: None,
            last_auto_save: None,
            device_serial: None,
            dirty: AtomicBool::new(false),
            baseline: None,
        }
    }

    pub fn mark_dirty(&self) {
        self.dirty.store(true, Ordering::Release);
    }

    pub fn on_session_connect(&mut self, serial: &str) {
        self.capture_enabled = true;
        self.device_serial = Some(serial.to_string());
        self.skip_next_sample = true;
        self.status = "Waiting for first fresh spectrum sample.".into();
        self.mark_dirty();
    }

    pub fn on_reconnect(&mut self) {
        self.skip_next_sample = true;
        self.reconnect_baseline_pending = true;
        self.baseline = None;
        self.last_ingest_at = None;
        self.status =
            "Reconnecting. Next sample re-baselines; offline counts become a gap row.".into();
        self.mark_dirty();
    }

    pub fn on_disconnect(&mut self) {
        self.capture_enabled = false;
        self.live_series = None;
        self.last_ingested_sequence = 0;
        self.skip_next_sample = false;
        self.reconnect_baseline_pending = false;
        self.last_ingest_at = None;
        self.baseline = None;
        self.last_auto_save = None;
        self.device_serial = None;
        self.mark_dirty();
    }

    pub fn ingest_spectrum(&mut self, spectrum: &SpectrumView) {
        if !self.capture_enabled {
            return;
        }
        let sequence = self.last_ingested_sequence.saturating_add(1);
        ingest::ingest_capture(self, spectrum, sequence);
    }

    pub fn maybe_auto_save(&mut self) {
        ingest::maybe_auto_save_capture(self);
    }
}

pub fn spawn_capture_router(
    worker_events: Receiver<crate::worker::WorkerEvent>,
    ui_events: Sender<crate::worker::WorkerEvent>,
    capture: Arc<std::sync::Mutex<SpectrogramCapture>>,
) {
    std::thread::spawn(move || {
        debug!("spectrogram capture router ready");
        while let Ok(event) = worker_events.recv() {
            match &event {
                crate::worker::WorkerEvent::Spectrum(spectrum) => {
                    if let Ok(mut cap) = capture.lock() {
                        cap.ingest_spectrum(spectrum);
                        cap.maybe_auto_save();
                    }
                }
                crate::worker::WorkerEvent::Connected(info) => {
                    if let Ok(mut cap) = capture.lock() {
                        cap.on_session_connect(&info.serial);
                    }
                }
                crate::worker::WorkerEvent::Reconnecting => {
                    if let Ok(mut cap) = capture.lock() {
                        cap.on_reconnect();
                    }
                }
                crate::worker::WorkerEvent::Disconnected => {
                    if let Ok(mut cap) = capture.lock() {
                        cap.on_disconnect();
                    }
                }
                _ => {}
            }
            if ui_events.send(event).is_err() {
                break;
            }
        }
        info!("spectrogram capture router stopped");
    });
}
