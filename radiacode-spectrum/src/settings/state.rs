use radiacode_core::DeviceConfig;

use crate::app_config::{load_app_config, save_app_config, AppConfig};
use crate::spectrogram::settings::{load_settings, save_settings, SpectrogramSettings};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsDeviceOp {
    Idle,
    Loading,
    Saving,
}

pub struct SettingsState {
    pub baseline: Option<DeviceConfig>,
    pub draft: Option<DeviceConfig>,
    pub device_op: SettingsDeviceOp,
    pub show_load_confirm: bool,
    pub app: AppConfig,
    pub spectrogram: SpectrogramSettings,
    pub status: String,
}

impl SettingsState {
    pub fn new() -> Self {
        Self {
            baseline: None,
            draft: None,
            device_op: SettingsDeviceOp::Idle,
            show_load_confirm: false,
            app: load_app_config(),
            spectrogram: load_settings(),
            status: String::new(),
        }
    }

    pub fn device_busy(&self) -> bool {
        self.device_op != SettingsDeviceOp::Idle
    }

    pub fn on_loaded(&mut self, config: DeviceConfig) {
        self.baseline = Some(config);
        self.draft = Some(config);
        self.device_op = SettingsDeviceOp::Idle;
        self.show_load_confirm = false;
        self.status.clear();
    }

    pub fn on_saved(&mut self, config: DeviceConfig) {
        self.baseline = Some(config);
        self.draft = Some(config);
        self.device_op = SettingsDeviceOp::Idle;
        self.show_load_confirm = false;
        self.status = "Saved to device".into();
    }

    pub fn on_device_op_failed(&mut self, message: String) {
        self.device_op = SettingsDeviceOp::Idle;
        self.status = message;
    }

    pub fn on_disconnect(&mut self) {
        self.baseline = None;
        self.draft = None;
        self.device_op = SettingsDeviceOp::Idle;
        self.show_load_confirm = false;
        self.status.clear();
    }

    pub fn draft_dirty(&self) -> bool {
        match (self.baseline.as_ref(), self.draft.as_ref()) {
            (Some(baseline), Some(draft)) => baseline != draft,
            _ => false,
        }
    }

    pub fn needs_auto_load(&self) -> bool {
        self.draft.is_none()
            && self.device_op == SettingsDeviceOp::Idle
            && !self.show_load_confirm
    }

    pub fn begin_load(&mut self) {
        self.device_op = SettingsDeviceOp::Loading;
        self.show_load_confirm = false;
        self.status = "Loading from device…".into();
    }

    pub fn begin_save(&mut self) {
        self.device_op = SettingsDeviceOp::Saving;
        self.status = "Saving to device…".into();
    }

    pub fn request_load(&mut self) {
        if self.draft_dirty() {
            self.show_load_confirm = true;
        }
    }

    pub fn discard(&mut self) {
        if let Some(baseline) = self.baseline {
            self.draft = Some(baseline);
        }
        self.show_load_confirm = false;
        self.status.clear();
    }

    pub fn persist_app(&mut self) {
        self.app.clamp();
        if let Err(error) = save_app_config(&self.app) {
            self.status = format!("Failed to save app settings: {error}");
        }
    }

    pub fn persist_spectrogram(&mut self) {
        self.spectrogram.clamp();
        if let Err(error) = save_settings(&self.spectrogram) {
            self.status = format!("Failed to save capture settings: {error}");
        }
    }

    pub fn apply_spectrogram_to(&self, target: &mut SpectrogramSettings) {
        *target = self.spectrogram.clone();
    }
}
