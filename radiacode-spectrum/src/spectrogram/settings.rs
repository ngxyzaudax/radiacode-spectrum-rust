use std::fs;
use std::path::PathBuf;

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use crate::spectrogram::color_scheme::ColorScheme;

pub const DEFAULT_CAPTURE_INTERVAL_SECS: f64 = 5.0;
pub const DEFAULT_MAX_SAMPLES: usize = 10_000;
pub const MIN_CAPTURE_INTERVAL_SECS: f64 = 1.0;
pub const MAX_CAPTURE_INTERVAL_SECS: f64 = 600.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpectrogramSettings {
    pub capture_interval_secs: f64,
    pub max_samples: usize,
    pub z_min: f32,
    pub z_max: f32,
    pub auto_brightness: bool,
    pub palette: ColorScheme,
    pub newest_at_bottom: bool,
    #[serde(default)]
    pub recordings_dir: String,
}

impl Default for SpectrogramSettings {
    fn default() -> Self {
        Self {
            capture_interval_secs: DEFAULT_CAPTURE_INTERVAL_SECS,
            max_samples: DEFAULT_MAX_SAMPLES,
            z_min: 0.0,
            z_max: 100.0,
            auto_brightness: true,
            palette: ColorScheme::Viridis,
            newest_at_bottom: true,
            recordings_dir: String::new(),
        }
    }
}

impl SpectrogramSettings {
    pub fn clamp(&mut self) {
        self.capture_interval_secs = self
            .capture_interval_secs
            .clamp(MIN_CAPTURE_INTERVAL_SECS, MAX_CAPTURE_INTERVAL_SECS);
        self.max_samples = self.max_samples.clamp(100, 20_000);
        if self.z_max <= self.z_min {
            self.z_max = self.z_min + 1.0;
        }
    }

    pub fn capture_interval(&self) -> f64 {
        self.capture_interval_secs
    }
}

pub fn settings_path() -> PathBuf {
    ProjectDirs::from("com", "radiacode", "radiacode-spectrum")
        .map(|dirs| dirs.data_dir().join("spectrogram_settings.json"))
        .unwrap_or_else(|| PathBuf::from("spectrogram_settings.json"))
}

pub fn load_settings() -> SpectrogramSettings {
    let path = settings_path();
    let Ok(bytes) = fs::read(&path) else {
        return SpectrogramSettings::default();
    };
    serde_json::from_slice(&bytes).unwrap_or_default()
}

pub fn save_settings(settings: &SpectrogramSettings) -> std::io::Result<()> {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec_pretty(settings)?;
    fs::write(path, bytes)
}
