use std::sync::mpsc::{self, Receiver};

use egui::{Context, RichText};
use radiacode_core::DeviceEndpoint;
use radiacode_usb::{access_status, install_access_rule, UsbAccessStatus};
use tracing::info;

use crate::theme::ACCENT;

pub struct UsbAccessPrompt {
    pub endpoint: DeviceEndpoint,
    pub status: UsbAccessStatus,
    pub installing: bool,
    pub message: String,
    install_rx: Option<Receiver<Result<(), String>>>,
}

impl UsbAccessPrompt {
    pub fn new(endpoint: DeviceEndpoint, status: UsbAccessStatus) -> Self {
        Self {
            endpoint,
            status,
            installing: false,
            message: prompt_message(status),
            install_rx: None,
        }
    }

    pub fn refresh_status(&mut self) {
        self.status = access_status();
        self.message = prompt_message(self.status);
    }

    pub fn poll_install(&mut self) -> Option<UsbAccessOutcome> {
        let Some(rx) = self.install_rx.as_ref() else {
            return None;
        };
        match rx.try_recv() {
            Ok(Ok(())) => {
                self.installing = false;
                self.install_rx = None;
                self.refresh_status();
                Some(UsbAccessOutcome::Installed {
                    endpoint: self.endpoint.clone(),
                    need_replug: self.status == UsbAccessStatus::RuleInstalledNeedReplug,
                })
            }
            Ok(Err(error)) => {
                self.installing = false;
                self.install_rx = None;
                self.message = error;
                None
            }
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.installing = false;
                self.install_rx = None;
                self.message = "USB setup stopped unexpectedly.".into();
                None
            }
        }
    }

    pub fn start_install(&mut self) {
        if self.installing {
            return;
        }
        info!("starting usb access rule install via pkexec");
        let (tx, rx) = mpsc::channel();
        self.install_rx = Some(rx);
        self.installing = true;
        self.message = "Waiting for system authentication…".into();
        std::thread::spawn(move || {
            let result = install_access_rule();
            let _ = tx.send(result);
        });
    }
}

pub enum UsbAccessOutcome {
    Installed {
        endpoint: DeviceEndpoint,
        need_replug: bool,
    },
}

pub enum UsbAccessAction {
    Install,
    RescanAndConnect,
    Dismiss,
}

pub fn usb_access_required(endpoint: &DeviceEndpoint) -> Option<UsbAccessStatus> {
    if !matches!(endpoint, DeviceEndpoint::Usb { .. }) {
        return None;
    }
    match access_status() {
        UsbAccessStatus::Granted => None,
        status => Some(status),
    }
}

pub fn draw_usb_access_dialog(
    ctx: &Context,
    prompt: &mut UsbAccessPrompt,
) -> Option<UsbAccessAction> {
    let mut action = None;
    let open = egui::Window::new("USB access required")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label(
                RichText::new("Radiacode needs permission to open the USB device.")
                    .strong()
                    .color(ACCENT),
            );
            ui.add_space(8.0);
            ui.label(&prompt.message);
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                let can_install = !prompt.installing
                    && prompt.status != UsbAccessStatus::RuleInstalledNeedReplug;
                if ui
                    .add_enabled(can_install, egui::Button::new("Grant USB access"))
                    .clicked()
                {
                    action = Some(UsbAccessAction::Install);
                }
                if prompt.status == UsbAccessStatus::RuleInstalledNeedReplug
                    && ui
                        .add_enabled(!prompt.installing, egui::Button::new("Rescan and connect"))
                        .clicked()
                {
                    action = Some(UsbAccessAction::RescanAndConnect);
                }
                if ui.button("Cancel").clicked() {
                    action = Some(UsbAccessAction::Dismiss);
                }
            });
            if prompt.installing {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("System password dialog should appear…");
                });
            }
        });
    if open.is_none() {
        action = Some(UsbAccessAction::Dismiss);
    }
    action
}

fn prompt_message(status: UsbAccessStatus) -> String {
    match status {
        UsbAccessStatus::Granted => "USB access is available.".into(),
        UsbAccessStatus::RuleMissing => {
            "Linux blocked USB access. Grant access once to install a udev rule, then replug the device.".into()
        }
        UsbAccessStatus::RuleInstalledNeedReplug => {
            "Unplug and replug the Radiacode, then press Rescan and connect.".into()
        }
    }
}
