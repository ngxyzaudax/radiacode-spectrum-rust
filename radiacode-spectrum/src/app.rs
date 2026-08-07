use std::sync::{Arc, Mutex};
use std::time::Duration;

use eframe::App;
use egui::{CentralPanel, Context, Panel, Ui, ViewportCommand, ViewportId};
use radiacode_core::{merge_discovered, resolve_usb_endpoint, DeviceEndpoint, TransportKind};
use tracing::{debug, info, warn};

use crate::dosimeter::{draw_dosimeter_controls, draw_dosimeter_view, DosimeterAction};
use crate::events::AppState;
use crate::icon::app_icon;
use crate::model::{ConnectionState, DeviceInfo};
use crate::monitor::{draw_monitor_controls, draw_monitor_view, AlarmLevel};
use crate::pc_alarm::maybe_beep_alarm;
use crate::scale::YScale;
use crate::settings::{draw_settings_view, SettingsAction, SettingsDeviceOp, SettingsState};
use crate::spectrogram::capture::SpectrogramCapture;
use crate::spectrogram::ui_controls::{draw_spectrogram_controls, SpectrogramControlsAction};
use crate::spectrogram::ui_view::draw_spectrogram_view;
use crate::spectrogram::SpectrogramState;
use crate::theme;
use crate::ui_controls::{draw_spectrum_controls, ControlsAction, ControlsProps};
use crate::ui_device::{draw_device_panel, DeviceAction, DevicePanelProps};
use crate::ui_disconnected::{draw_disconnected_view, shows_tab_content};
use crate::ui_plot::draw_spectrum_plot;
use crate::usb_access::{
    draw_usb_access_dialog, usb_access_required, UsbAccessAction, UsbAccessOutcome, UsbAccessPrompt,
};
use crate::view_tab::ViewTab;
use crate::worker::{spawn_worker, WorkerCommand, WorkerEvent, WorkerHandle};

pub struct SpectrumApp {
    worker: WorkerHandle,
    state: AppState,
    settings: SettingsState,
    spectrogram: SpectrogramState,
    active_tab: ViewTab,
    previous_tab: ViewTab,
    y_scale: YScale,
    smooth_window: usize,
    theme_ready: bool,
    startup_scan_sent: bool,
    icon_sent: bool,
    session_blocked: bool,
    usb_access_prompt: Option<UsbAccessPrompt>,
    last_alarm_level: AlarmLevel,
    auto_connect_attempted: bool,
}

impl SpectrumApp {
    pub fn new() -> Self {
        let capture = Arc::new(Mutex::new(SpectrogramCapture::new()));
        let settings = SettingsState::new();
        let mut spectrogram = SpectrogramState::new(Arc::clone(&capture));
        spectrogram.settings = settings.spectrogram.clone();
        spectrogram.on_settings_changed();
        spectrogram.refresh_history();
        Self {
            worker: spawn_worker(capture),
            state: AppState::new(),
            settings,
            spectrogram,
            active_tab: ViewTab::Monitor,
            previous_tab: ViewTab::Monitor,
            y_scale: YScale::Linear,
            smooth_window: 1,
            theme_ready: false,
            startup_scan_sent: false,
            icon_sent: false,
            session_blocked: false,
            usb_access_prompt: None,
            last_alarm_level: AlarmLevel::Normal,
            auto_connect_attempted: false,
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
            warn!("device worker stopped");
            self.state.status = "Device worker stopped.".into();
        }
    }

    fn schedule_spectrum(&mut self) {
        if self.state.try_schedule_spectrum() {
            self.send(WorkerCommand::FetchSpectrum);
        }
    }

    fn sync_capture_interval(&mut self) {
        self.send(WorkerCommand::SetCaptureInterval(
            self.spectrogram.settings.capture_interval_secs,
        ));
    }

    fn sync_monitor_poll_interval(&mut self) {
        self.send(WorkerCommand::SetMonitorPollInterval(
            self.settings.app.monitor_poll_secs,
        ));
    }

    fn schedule_monitor(&mut self) {
        if self.state.try_schedule_monitor() {
            self.send(WorkerCommand::FetchMonitor);
        }
    }

    fn endpoint_from_info(info: &DeviceInfo) -> DeviceEndpoint {
        match info.transport {
            TransportKind::Bluetooth => DeviceEndpoint::Bluetooth {
                address: info.address.clone(),
            },
            TransportKind::Usb => DeviceEndpoint::Usb {
                serial: info.address.clone(),
            },
        }
    }

    fn remember_connected_device(&mut self, info: &DeviceInfo) {
        if !self.settings.app.remember_device {
            return;
        }
        self.settings.app.last_endpoint = Some(Self::endpoint_from_info(info));
        self.settings.persist_app();
    }

    fn maybe_auto_connect(&mut self) {
        if self.auto_connect_attempted {
            return;
        }
        if !self.settings.app.auto_connect {
            return;
        }
        if self.state.connection != ConnectionState::Disconnected {
            return;
        }
        let Some(endpoint) = self.settings.app.last_endpoint.clone() else {
            return;
        };
        let found = self
            .state
            .devices
            .iter()
            .any(|device| device.endpoint == endpoint);
        if !found {
            return;
        }
        if usb_access_required(&endpoint).is_some() {
            info!(?endpoint, "auto-connect skipped: usb access required");
            return;
        }
        self.auto_connect_attempted = true;
        info!(?endpoint, "auto-connecting to remembered device");
        self.start_connect(endpoint);
    }

    fn poll_events(&mut self) {
        while let Ok(event) = self.worker.events.try_recv() {
            if let WorkerEvent::UsbPermissionRequired { endpoint } = &event {
                if let Some(status) = usb_access_required(endpoint) {
                    self.usb_access_prompt =
                        Some(UsbAccessPrompt::new(endpoint.clone(), status));
                }
            }
            if let WorkerEvent::DeviceConfig(config) = &event {
                if !self.session_blocked {
                    match self.settings.device_op {
                        SettingsDeviceOp::Loading => self.settings.on_loaded(*config),
                        SettingsDeviceOp::Saving => self.settings.on_saved(*config),
                        SettingsDeviceOp::Idle => {}
                    }
                    self.state.monitor.apply_limits(config.alarms);
                }
            }
            if let WorkerEvent::Error(message) = &event {
                if self.settings.device_op == SettingsDeviceOp::Loading
                    || self.settings.device_op == SettingsDeviceOp::Saving
                {
                    self.settings.on_device_op_failed(message.clone());
                } else if self.settings.status == "Saved to device" {
                    self.settings.status = message.clone();
                }
            }
            let fetch_spectrum =
                matches!(&event, WorkerEvent::Connected(_)) && !self.session_blocked;
            let initial_connect = matches!(&event, WorkerEvent::Connected(_))
                && self.state.connection == ConnectionState::Connecting;
            let scan_finished = matches!(&event, WorkerEvent::ScanFinished(_));
            let monitor_sample = matches!(&event, WorkerEvent::MonitorSample(_));
            match &event {
                WorkerEvent::Reconnecting if !self.session_blocked => {
                    self.spectrogram.on_reconnect();
                }
                WorkerEvent::Connected(info) if !self.session_blocked && initial_connect => {
                    self.sync_capture_interval();
                    self.sync_monitor_poll_interval();
                    self.spectrogram.sync_from_capture();
                    self.remember_connected_device(info);
                    if self.active_tab == ViewTab::Settings && !self.settings.draft_dirty() {
                        self.start_device_load();
                    }
                }
                WorkerEvent::Disconnected => {
                    self.spectrogram.on_disconnect();
                    self.settings.on_disconnect();
                    self.session_blocked = false;
                    self.last_alarm_level = AlarmLevel::Normal;
                }
                _ => {}
            }
            if let Some(command) = self.state.apply_event(event, !self.session_blocked) {
                match command {
                    WorkerCommand::FetchMonitor => self.schedule_monitor(),
                    other => self.send(other),
                }
            }
            if monitor_sample && !self.session_blocked {
                self.check_pc_alarm();
            }
            if fetch_spectrum {
                self.schedule_spectrum();
            }
            if scan_finished {
                self.maybe_auto_connect();
            }
        }
    }

    fn check_pc_alarm(&mut self) {
        let dose = self.state.monitor.dose_alarm_level();
        let count = self.state.monitor.count_alarm_level();
        let accum = self.state.dosimeter.dose_alarm_level();
        let current = dose.max(count).max(accum);
        let rising = current > self.last_alarm_level && current > AlarmLevel::Normal;
        self.last_alarm_level = current;
        if rising {
            maybe_beep_alarm(self.settings.app.pc_alarm_repeat);
        }
    }

    fn start_scan(&mut self) {
        info!("ui starting scan");
        self.state.scanning = true;
        self.state.status = "Scanning for RadiaCode devices…".into();
        self.send(WorkerCommand::Scan);
    }

    fn refresh_usb_devices(&mut self) {
        let bluetooth: Vec<_> = self
            .state
            .devices
            .iter()
            .filter(|device| device.endpoint.transport() == TransportKind::Bluetooth)
            .cloned()
            .collect();
        match radiacode_usb::scan_usb_devices() {
            Ok(usb) => {
                self.state.devices = merge_discovered(usb, bluetooth);
            }
            Err(error) => {
                warn!(%error, "usb rescan failed");
            }
        }
    }

    fn start_connect(&mut self, endpoint: DeviceEndpoint) {
        self.start_connect_internal(endpoint, false);
    }

    fn start_connect_internal(&mut self, endpoint: DeviceEndpoint, force_usb: bool) {
        let endpoint = resolve_usb_endpoint(&self.state.devices, &endpoint);
        if !force_usb {
            if let Some(status) = usb_access_required(&endpoint) {
                info!(?endpoint, ?status, "usb access required before connect");
                self.session_blocked = false;
                self.state.connection = ConnectionState::Disconnected;
                self.state.connecting_endpoint = Some(endpoint.clone());
                self.state.status = "USB access required.".into();
                self.usb_access_prompt = Some(UsbAccessPrompt::new(endpoint, status));
                return;
            }
        }
        let address = endpoint.address_label().to_string();
        info!(%address, ?endpoint, force_usb, "ui starting connect");
        self.session_blocked = false;
        self.state.connection = ConnectionState::Connecting;
        self.state.connecting_endpoint = Some(endpoint.clone());
        self.state.status = format!("Connecting to {address}…");
        let hint_rssi = self.hint_rssi_for_endpoint(&endpoint);
        self.send(WorkerCommand::Connect {
            endpoint,
            hint_rssi,
        });
    }

    fn hint_rssi_for_endpoint(&self, endpoint: &DeviceEndpoint) -> Option<i16> {
        self.state
            .devices
            .iter()
            .find(|device| device.endpoint == *endpoint)
            .and_then(|device| device.rssi)
    }

    fn disconnect_device(&mut self) {
        info!("ui disconnect requested");
        self.worker.end_session();
        self.session_blocked = true;
        self.state.clear_session();
        self.spectrogram.on_disconnect();
        self.settings.on_disconnect();
        self.last_alarm_level = AlarmLevel::Normal;
        self.send(WorkerCommand::Disconnect);
    }

    fn handle_device_action(&mut self, action: DeviceAction) {
        match action {
            DeviceAction::Scan => self.start_scan(),
            DeviceAction::Connect(endpoint) => self.start_connect(endpoint),
            DeviceAction::Disconnect => self.disconnect_device(),
        }
    }

    fn handle_controls_action(&mut self, action: ControlsAction) {
        if matches!(action, ControlsAction::Reset) {
            self.state.spectrum_fetch_pending = true;
            self.send(WorkerCommand::ResetSpectrum);
        }
    }

    fn handle_dosimeter_action(&mut self, action: DosimeterAction) {
        if matches!(action, DosimeterAction::ResetDose) {
            self.send(WorkerCommand::ResetDose);
        }
    }

    fn handle_settings_action(&mut self, action: SettingsAction) {
        match action {
            SettingsAction::LoadDevice => {
                if self.settings.draft_dirty() {
                    self.settings.request_load();
                } else {
                    self.start_device_load();
                }
            }
            SettingsAction::ConfirmLoad => {
                self.start_device_load();
            }
            SettingsAction::CancelLoad => {}
            SettingsAction::SaveDevice => {
                let Some(draft) = self.settings.draft else {
                    return;
                };
                self.settings.begin_save();
                self.send(WorkerCommand::ApplyDeviceConfig(draft));
            }
            SettingsAction::DiscardChanges => {
                self.settings.discard();
            }
            SettingsAction::SyncClock => {
                self.settings.status = "Syncing clock…".into();
                self.send(WorkerCommand::SyncDeviceClock);
            }
            SettingsAction::AppChanged => {
                self.settings.persist_app();
                self.sync_monitor_poll_interval();
            }
            SettingsAction::SpectrogramChanged => {
                self.settings.persist_spectrogram();
                self.settings
                    .apply_spectrogram_to(&mut self.spectrogram.settings);
                self.spectrogram.on_settings_changed();
                self.sync_capture_interval();
            }
        }
    }

    fn start_device_load(&mut self) {
        if self.state.connection != ConnectionState::Connected {
            return;
        }
        if self.settings.device_busy() {
            return;
        }
        self.settings.begin_load();
        self.send(WorkerCommand::FetchDeviceConfig);
    }

    fn maybe_settings_auto_load(&mut self) {
        if self.active_tab != ViewTab::Settings {
            return;
        }
        if self.state.connection != ConnectionState::Connected {
            return;
        }
        if !self.settings.needs_auto_load() {
            return;
        }
        self.start_device_load();
    }

    fn enter_settings_tab(&mut self) {
        if self.state.connection != ConnectionState::Connected {
            return;
        }
        if self.settings.draft_dirty() {
            self.settings.show_load_confirm = true;
            return;
        }
        self.start_device_load();
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
            SpectrogramControlsAction::SettingsChanged => {}
            SpectrogramControlsAction::LibraryChanged => {}
        }
    }

    fn enter_spectrum_tab(&mut self) {}

    fn enter_spectrogram_tab(&mut self) {
        info!("entered spectrogram tab");
        self.spectrogram.on_tab_enter();
    }

    fn poll_usb_access(&mut self) {
        let Some(prompt) = self.usb_access_prompt.as_mut() else {
            return;
        };
        if let Some(outcome) = prompt.poll_install() {
            match outcome {
                UsbAccessOutcome::Installed { endpoint, need_replug } => {
                    let message = prompt.message.clone();
                    self.start_scan();
                    if need_replug {
                        self.state.status = message;
                    } else {
                        self.usb_access_prompt = None;
                        self.refresh_usb_devices();
                        let endpoint = resolve_usb_endpoint(&self.state.devices, &endpoint);
                        self.start_connect_internal(endpoint, true);
                    }
                }
            }
        }
    }

    fn handle_usb_access_action(&mut self, action: UsbAccessAction) {
        let Some(prompt) = self.usb_access_prompt.as_mut() else {
            return;
        };
        match action {
            UsbAccessAction::Install => prompt.start_install(),
            UsbAccessAction::RescanAndConnect => {
                prompt.refresh_status();
                let preferred = prompt.endpoint.clone();
                self.usb_access_prompt = None;
                self.refresh_usb_devices();
                let endpoint = resolve_usb_endpoint(&self.state.devices, &preferred);
                self.start_connect_internal(endpoint, true);
            }
            UsbAccessAction::Dismiss => {
                self.usb_access_prompt = None;
                self.state.connecting_endpoint = None;
                if self.state.status == "USB access required." {
                    self.state.status = "USB setup cancelled.".into();
                }
            }
        }
    }

    fn maybe_live_refresh(&mut self) {
        if self.state.connection != ConnectionState::Connected {
            return;
        }
        if self.active_tab == ViewTab::Spectrum
            && self
                .state
                .live_refresh_due(true, self.settings.app.spectrum_refresh_secs)
        {
            debug!("spectrum tab live refresh due");
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
            self.sync_monitor_poll_interval();
            self.sync_capture_interval();
        }
        if !self.startup_scan_sent {
            self.startup_scan_sent = true;
            self.start_scan();
        }
        self.poll_events();
        self.poll_usb_access();
        if self.state.connection == ConnectionState::Connected {
            self.spectrogram.sync_from_capture();
        }
        self.maybe_live_refresh();
        self.maybe_settings_auto_load();
        ctx.request_repaint_after(Duration::from_millis(200));
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if let Some(prompt) = self.usb_access_prompt.as_mut() {
            if let Some(action) = draw_usb_access_dialog(&ctx, prompt) {
                self.handle_usb_access_action(action);
            }
        }
        Panel::left("sidebar")
            .resizable(true)
            .default_size(300.0)
            .show(ui, |ui| {
                let device_action = draw_device_panel(
                    ui,
                    DevicePanelProps {
                        devices: &self.state.devices,
                        connection: self.state.connection,
                        connecting_endpoint: self.state.connecting_endpoint.as_ref(),
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
                        ViewTab::Monitor => draw_monitor_controls(ui),
                        ViewTab::Spectrum => {
                            if let Some(action) = draw_spectrum_controls(
                                ui,
                                ControlsProps {
                                    connection: self.state.connection,
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
                        ViewTab::Dosimeter => {
                            if let Some(action) = draw_dosimeter_controls(ui) {
                                self.handle_dosimeter_action(action);
                            }
                        }
                        ViewTab::Settings => {}
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
                ui.selectable_value(
                    &mut self.active_tab,
                    ViewTab::Dosimeter,
                    ViewTab::Dosimeter.label(),
                );
                ui.selectable_value(
                    &mut self.active_tab,
                    ViewTab::Settings,
                    ViewTab::Settings.label(),
                );
            });
            if self.active_tab == ViewTab::Spectrum && previous_tab != ViewTab::Spectrum {
                self.enter_spectrum_tab();
            }
            if self.active_tab == ViewTab::Spectrogram && previous_tab != ViewTab::Spectrogram {
                self.enter_spectrogram_tab();
            }
            if self.active_tab == ViewTab::Settings && previous_tab != ViewTab::Settings {
                self.enter_settings_tab();
            }
            self.previous_tab = self.active_tab;
            ui.separator();
            if self.active_tab == ViewTab::Settings {
                let content_rect = ui.available_rect_before_wrap();
                ui.scope_builder(egui::UiBuilder::new().max_rect(content_rect), |ui| {
                    ui.set_clip_rect(content_rect);
                    if let Some(action) = draw_settings_view(
                        ui,
                        &mut self.settings,
                        self.state.connection,
                        self.state.device_info.as_ref(),
                    ) {
                        self.handle_settings_action(action);
                    }
                });
            } else if shows_tab_content(self.state.connection) {
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
                        ViewTab::Dosimeter => {
                            draw_dosimeter_view(ui, &self.state.dosimeter);
                        }
                        ViewTab::Settings => {}
                    }
                });
            } else {
                draw_disconnected_view(ui, self.state.connection);
            }
        });
    }
}
