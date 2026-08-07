use std::fs;
use std::path::PathBuf;

use directories::ProjectDirs;
use radiacode_core::DeviceEndpoint;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppConfig {
    pub monitor_poll_secs: u64,
    pub spectrum_refresh_secs: u64,
    pub remember_device: bool,
    pub last_endpoint: Option<DeviceEndpoint>,
    pub auto_connect: bool,
    pub pc_alarm_repeat: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            monitor_poll_secs: 1,
            spectrum_refresh_secs: 1,
            remember_device: true,
            last_endpoint: None,
            auto_connect: false,
            pc_alarm_repeat: false,
        }
    }
}

impl AppConfig {
    pub fn clamp(&mut self) {
        self.monitor_poll_secs = self.monitor_poll_secs.clamp(1, 60);
        self.spectrum_refresh_secs = self.spectrum_refresh_secs.clamp(1, 60);
    }
}

pub fn config_path() -> PathBuf {
    ProjectDirs::from("com", "radiacode", "radiacode-spectrum")
        .map(|dirs| dirs.data_dir().join("app_config.json"))
        .unwrap_or_else(|| PathBuf::from("app_config.json"))
}

pub fn load_app_config() -> AppConfig {
    let Ok(bytes) = fs::read(config_path()) else {
        return AppConfig::default();
    };
    let mut config: AppConfig = serde_json::from_slice(&bytes).unwrap_or_default();
    config.clamp();
    config
}

pub fn save_app_config(config: &AppConfig) -> std::io::Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut config = config.clone();
    config.clamp();
    fs::write(path, serde_json::to_vec_pretty(&config)?)
}
