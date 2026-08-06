use std::time::Duration;

use eframe::App;
use egui::{CentralPanel, Context, Panel, Ui, ViewportCommand, ViewportId};
use tracing::{debug, info, warn};

use crate::events::AppState;
use crate::icon::app_icon;
use crate::model::ConnectionState;
use crate::monitor::{draw_monitor_controls, draw_monitor_view, MonitorControlsAction};
use crate::scale::YScale;
use crate::spectrogram::ui_controls::{draw_spectrogram_controls, SpectrogramControlsAction};
use crate::spectrogram::ui_view::draw_spectrogram_view;
use crate::spectrogram::SpectrogramState;
use crate::theme;
use crate::ui_controls::{draw_spectrum_controls, ControlsAction, ControlsProps};
use crate::ui_device::{draw_device_panel, DeviceAction, DevicePanelProps};
use crate::ui_disconnected::{draw_disconnected_view, shows_tab_content};
use crate::ui_plot::draw_spectrum_plot;
use crate::view_tab::ViewTab;
use crate::worker::{spawn_worker, WorkerCommand, WorkerEvent, WorkerHandle};

const MONITOR_POLL_SECS: u64 = 1;
const STATUS_POLL_SECS: u64 = 5;

pub struct SpectrumApp {
    worker: WorkerHandle,
    state: AppState,
    spectrogram: SpectrogramState,
    active_tab: ViewTab,
    previous_tab: ViewTab,
    live: bool,
    y_scale: YScale,
    smooth_window: usize,
    theme_ready: bool,
    startup_scan_sent: bool,
    icon_sent: bool,
    session_blocked: bool,
}

impl SpectrumApp {
    pub fn new() -> Self {
        let mut spectrogram = SpectrogramState::new();
        spectrogram.refresh_history();
        Self {
            worker: spawn_worker(),
            state: AppState::new(),
            spectrogram,
            active_tab: ViewTab::Monitor,
            previous_tab: ViewTab::Monitor,
            live: true,
            y_scale: YScale::Linear,
            smooth_window: 1,
            theme_ready: false,
            startup_scan_sent: false,
            icon_sent: false,
            session_blocked: false,
        }
    }

    fn ensure_window_icon(&mut self, ctx: &Context) {
        if self.icon_sent {
            return;
        }
        ctx.send_viewport_cmd_to(
            ViewportId::ROOT,
            ViewportCommand::Icon(Some(app_icon())),
        );
        self.icon_sent = true;
    }

    fn send(&mut self, command: WorkerCommand) {
        debug!(?command, "sending worker command");
        if self.worker.commands.send(command).is_err() {
            warn!("bluetooth worker stopped");
            self.state.status = "Bluetooth worker stopped.".into();
        }
    }

    fn schedule_spectrum(&mut self) {
        if self.state.try_schedule_spectrum() {
            self.send(WorkerCommand::FetchSpectrum);
        }
    }

    fn schedule_monitor(&mut self) {
        if self.state.try_schedule_monitor() {
            self.send(WorkerCommand::FetchMonitor);
        }
    }

    fn poll_events(&mut self) {
        while let Ok(event) = self.worker.events.try_recv() {
            let fetch_spectrum = matches!(&event, WorkerEvent::Connected(_)) && !self.session_blocked;
            let initial_connect =
                matches!(&event, WorkerEvent::Connected(_))
                    && self.state.connection == ConnectionState::Connecting;
            match &event {
                WorkerEvent::Reconnecting if !self.session_blocked => {
                    self.spectrogram.on_reconnect();
                }
                WorkerEvent::Connected(_) if !self.session_blocked && initial_connect => {
                    self.spectrogram.on_session_connect();
                }
                WorkerEvent::Disconnected => {
                    self.spectrogram.on_disconnect();
                    self.session_blocked = false;
                }
                _ => {}
            }
            if let Some(command) = self.state.apply_event(event, !self.session_blocked) {
                match command {
                    WorkerCommand::FetchMonitor => self.schedule_monitor(),
                    other => self.send(other),
                }
            }
            if fetch_spectrum {
                self.schedule_spectrum();
            }
        }
    }

    fn sync_spectrogram(&mut self) {
        let Some(spectrum) = self.state.spectrum.as_ref() else {
            return;
        };
        let serial = self
            .state
            .device_info
            .as_ref()
            .map(|info| info.serial.as_str());
        self.spectrogram.ingest_spectrum(
            spectrum,
            serial,
            self.state.spectrum_sequence,
            self.active_tab,
        );
        self.spectrogram.maybe_auto_save();
    }

    fn start_scan(&mut self) {
        info!("ui starting scan");
        self.state.scanning = true;
        self.state.status = "Scanning for RadiaCode devices…".into();
        self.send(WorkerCommand::Scan);
    }

    fn start_connect(&mut self, mac: String) {
        info!(%mac, "ui starting connect");
        self.session_blocked = false;
        self.state.connection = ConnectionState::Connecting;
        self.state.connecting_mac = Some(mac.clone());
        self.state.status = format!("Connecting to {mac}…");
        self.send(WorkerCommand::Connect(mac));
    }

    fn disconnect_device(&mut self) {
        info!("ui disconnect requested");
        self.worker.end_session();
        self.session_blocked = true;
        self.state.clear_session();
        self.spectrogram.on_disconnect();
        self.send(WorkerCommand::Disconnect);
    }

    fn handle_device_action(&mut self, action: DeviceAction) {
        match action {
            DeviceAction::Scan => self.start_scan(),
            DeviceAction::Connect(mac) => self.start_connect(mac),
            DeviceAction::Disconnect => self.disconnect_device(),
        }
    }

    fn handle_controls_action(&mut self, action: ControlsAction) {
        if matches!(action, ControlsAction::Reset) {
            self.state.spectrum_fetch_pending = true;
            self.send(WorkerCommand::ResetSpectrum);
        }
    }

    fn handle_monitor_action(&mut self, action: MonitorControlsAction) {
        match action {
            MonitorControlsAction::ApplyLimits => {
                self.send(WorkerCommand::SetAlarmLimits(
                    self.state.monitor.to_update(),
                ));
            }
        }
    }

    fn handle_spectrogram_action(&mut self, action: SpectrogramControlsAction) {
        match action {
            SpectrogramControlsAction::StartRecording => {
                let serial = self
                    .state
                    .device_info
                    .as_ref()
                    .map(|info| info.serial.as_str());
                if let Err(message) = self
                    .spectrogram
                    .start_recording(self.state.spectrum.as_ref(), serial)
                {
                    self.spectrogram.status = message;
                }
            }
            SpectrogramControlsAction::StopRecording => {
                if let Err(message) = self.spectrogram.stop_recording() {
                    self.spectrogram.status = message;
                }
            }
            SpectrogramControlsAction::ResumeRecording => {
                let serial = self
                    .state
                    .device_info
                    .as_ref()
                    .map(|info| info.serial.as_str());
                if let Err(message) = self
                    .spectrogram
                    .resume_recording(self.state.spectrum.as_ref(), serial)
                {
                    self.spectrogram.status = message;
                }
            }
            SpectrogramControlsAction::CloseLoaded => self.spectrogram.close_loaded(),
            SpectrogramControlsAction::Load(path) => self.spectrogram.request_load(path),
            SpectrogramControlsAction::SettingsChanged | SpectrogramControlsAction::LibraryChanged => {}
        }
    }

    fn enter_spectrum_tab(&mut self) {
        if self.state.connection == ConnectionState::Connected {
            self.schedule_spectrum();
        }
    }

    fn enter_spectrogram_tab(&mut self) {
        info!("entered spectrogram tab");
        self.spectrogram.on_tab_enter();
        if self.state.connection == ConnectionState::Connected {
            self.schedule_spectrum();
        }
    }

    fn maybe_live_refresh(&mut self) {
        if self.state.connection != ConnectionState::Connected {
            return;
        }
        let monitor_interval = match self.active_tab {
            ViewTab::Monitor => MONITOR_POLL_SECS,
            _ => STATUS_POLL_SECS,
        };
        if self.state.monitor_refresh_due(true, monitor_interval) {
            debug!(interval = monitor_interval, "monitor refresh due");
            self.schedule_monitor();
        }
        let interval = self.spectrogram.settings.capture_interval_secs.round() as u64;
        if self.state.live_refresh_due(self.live, interval) {
            debug!(interval, "spectrum refresh due");
            self.schedule_spectrum();
        }
    }
}

impl App for SpectrumApp {
    fn logic(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.ensure_window_icon(ctx);
        if !self.theme_ready {
            theme::apply(ctx);
            self.theme_ready = true;
        }
        if !self.startup_scan_sent {
            self.startup_scan_sent = true;
            self.start_scan();
        }
        self.poll_events();
        if self.state.connection == ConnectionState::Connected {
            self.sync_spectrogram();
        }
        self.maybe_live_refresh();
        ctx.request_repaint_after(Duration::from_millis(200));
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        Panel::left("sidebar")
            .resizable(true)
            .default_size(300.0)
            .show(ui, |ui| {
                let device_action = draw_device_panel(
                    ui,
                    DevicePanelProps {
                        devices: &self.state.devices,
                        connection: self.state.connection,
                        connecting_mac: self.state.connecting_mac.as_deref(),
                        device_info: self.state.device_info.as_ref(),
                        scanning: self.state.scanning,
                        busy: self.state.busy,
                        scanned_once: self.state.scanned_once,
                        status: &self.state.status,
                    },
                );
                if let Some(action) = device_action {
                    self.handle_device_action(action);
                }

                if shows_tab_content(self.state.connection) {
                    match self.active_tab {
                        ViewTab::Monitor => {
                            if let Some(action) = draw_monitor_controls(
                                ui,
                                &mut self.state.monitor,
                                true,
                                self.state.busy,
                            ) {
                                self.handle_monitor_action(action);
                            }
                        }
                        ViewTab::Spectrum => {
                            if let Some(action) = draw_spectrum_controls(
                                ui,
                                ControlsProps {
                                    connection: self.state.connection,
                                    live: &mut self.live,
                                    y_scale: &mut self.y_scale,
                                    smooth_window: &mut self.smooth_window,
                                },
                            ) {
                                self.handle_controls_action(action);
                            }
                        }
                        ViewTab::Spectrogram => {
                            if let Some(action) = draw_spectrogram_controls(
                                ui,
                                &mut self.spectrogram,
                                self.state.connection,
                                self.state.busy,
                            ) {
                                self.handle_spectrogram_action(action);
                            }
                        }
                    }
                }
            });

        CentralPanel::default().show(ui, |ui| {
            let previous_tab = self.previous_tab;
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut self.active_tab,
                    ViewTab::Monitor,
                    ViewTab::Monitor.label(),
                );
                ui.selectable_value(
                    &mut self.active_tab,
                    ViewTab::Spectrum,
                    ViewTab::Spectrum.label(),
                );
                ui.selectable_value(
                    &mut self.active_tab,
                    ViewTab::Spectrogram,
                    ViewTab::Spectrogram.label(),
                );
            });
            if self.active_tab == ViewTab::Spectrum && previous_tab != ViewTab::Spectrum {
                self.enter_spectrum_tab();
            }
            if self.active_tab == ViewTab::Spectrogram && previous_tab != ViewTab::Spectrogram {
                self.enter_spectrogram_tab();
            }
            self.previous_tab = self.active_tab;
            ui.separator();
            if shows_tab_content(self.state.connection) {
                let content_rect = ui.available_rect_before_wrap();
                ui.scope_builder(egui::UiBuilder::new().max_rect(content_rect), |ui| {
                    ui.set_clip_rect(content_rect);
                    match self.active_tab {
                        ViewTab::Monitor => draw_monitor_view(ui, &self.state.monitor),
                        ViewTab::Spectrum => {
                            draw_spectrum_plot(
                                ui,
                                self.state.spectrum.as_ref(),
                                self.y_scale,
                                self.smooth_window,
                            );
                        }
                        ViewTab::Spectrogram => {
                            draw_spectrogram_view(ui, &ctx, &mut self.spectrogram);
                        }
                    }
                });
            } else {
                draw_disconnected_view(ui, self.state.connection);
            }
        });
    }
}
