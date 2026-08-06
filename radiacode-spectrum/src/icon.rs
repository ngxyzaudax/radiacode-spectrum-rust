use std::sync::Arc;

use eframe::icon_data::from_png_bytes;
use egui::IconData;
use tracing::{info, warn};

pub const APP_ID: &str = "radiacode-spectrum";

const ICON_PNG: &[u8] = include_bytes!("../assets/icon.png");

pub fn app_icon() -> Arc<IconData> {
    match from_png_bytes(ICON_PNG) {
        Ok(icon) => {
            info!(width = icon.width, height = icon.height, "application icon loaded");
            Arc::new(icon)
        }
        Err(error) => {
            warn!(%error, "failed to load application icon");
            Arc::new(IconData::default())
        }
    }
}
