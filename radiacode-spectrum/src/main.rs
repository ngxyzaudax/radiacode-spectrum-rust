mod about;
mod analysis;
mod dosimeter;
mod app;
mod app_config;
mod energy;
mod events;
mod icon;
mod logging;
mod model;
mod monitor;
mod pc_alarm;
mod scale;
mod settings;
mod smooth;
mod spectrogram;
mod theme;
mod ui_controls;
mod ui_device;
mod ui_device_status;
mod ui_disconnected;
mod ui_plot;
mod ui_recording_library;
mod ui_recording_search;
mod usb_access;
mod view_tab;
mod worker;
mod worker_ops;

use std::process::ExitCode;

use app::SpectrumApp;
use icon::{app_icon, APP_ID};
use tracing::{error, info};

fn main() -> ExitCode {
    logging::init();
    info!("radiacode-spectrum starting");
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 760.0])
            .with_title("Radiacode")
            .with_app_id(APP_ID)
            .with_icon(app_icon()),
        ..Default::default()
    };
    match eframe::run_native(
        APP_ID,
        options,
        Box::new(|_cc| Ok(Box::new(SpectrumApp::new()))),
    ) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            error!(%error, "failed to start gui");
            ExitCode::FAILURE
        }
    }
}
