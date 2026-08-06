use egui::{RichText, Ui};

use radiacode_core::{DeviceEndpoint, DiscoveredDevice, TransportKind};

use crate::model::{ConnectionState, DeviceInfo};
use crate::theme::{ACCENT, MUTED};
use crate::ui_device_status::draw_status_row;

pub struct DevicePanelProps<'a> {
    pub devices: &'a [DiscoveredDevice],
    pub connection: ConnectionState,
    pub connecting_endpoint: Option<&'a DeviceEndpoint>,
    pub device_info: Option<&'a DeviceInfo>,
    pub scanning: bool,
    pub busy: bool,
    pub scanned_once: bool,
    pub status: &'a str,
}

pub enum DeviceAction {
    Scan,
    Connect(DeviceEndpoint),
    Disconnect,
}

pub fn draw_device_panel(ui: &mut Ui, props: DevicePanelProps<'_>) -> Option<DeviceAction> {
    let mut action = None;
    ui.label(RichText::new("Radiacode").size(22.0).color(ACCENT).strong());
    ui.add_space(12.0);

    match props.connection {
        ConnectionState::Connected => {
            if let Some(info) = props.device_info {
                action = draw_connected(ui, info).or(action);
            }
        }
        ConnectionState::Connecting => {
            draw_connecting(ui, props.connecting_endpoint);
        }
        ConnectionState::Disconnected => {
            action = draw_discovery(ui, &props).or(action);
        }
    }

    ui.add_space(10.0);
    ui.label(RichText::new(props.status).small().color(MUTED));
    action
}

fn draw_connected(ui: &mut Ui, info: &DeviceInfo) -> Option<DeviceAction> {
    let mut action = None;
    ui.label(RichText::new("Connected").color(ACCENT).strong());
    ui.add_space(6.0);
    ui.label(format!("Model {}", info.model));
    ui.label(RichText::new(&info.serial).size(18.0).strong());
    ui.add_space(4.0);
    draw_status_row(ui, info);
    ui.add_space(4.0);
    ui.label(format!("Transport {}", info.transport_label()));
    ui.label(RichText::new(&info.address).monospace());
    ui.label(format!("Firmware {}", info.firmware));
    ui.label(format!(
        "Calibration  a0={:.2}  a1={:.3}  a2={:.5}",
        info.energy_calib[0], info.energy_calib[1], info.energy_calib[2]
    ));
    ui.add_space(10.0);
    if ui
        .add(egui::Button::new("Disconnect").min_size([ui.available_width(), 28.0].into()))
        .clicked()
    {
        action = Some(DeviceAction::Disconnect);
    }
    action
}

fn draw_connecting(ui: &mut Ui, endpoint: Option<&DeviceEndpoint>) {
    ui.label(RichText::new("Connecting").strong());
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.spinner();
        let label = endpoint
            .map(|value| value.address_label())
            .unwrap_or("device");
        ui.label(RichText::new(label).monospace());
    });
    ui.add_space(6.0);
    ui.label(RichText::new("Keep the detector powered on.").weak());
}

fn draw_discovery(ui: &mut Ui, props: &DevicePanelProps<'_>) -> Option<DeviceAction> {
    let mut action = None;
    ui.horizontal(|ui| {
        ui.label(RichText::new("Nearby devices").strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let label = if props.scanning { "Scanning…" } else { "Scan" };
            if ui
                .add_enabled(!props.busy && !props.scanning, egui::Button::new(label))
                .clicked()
            {
                action = Some(DeviceAction::Scan);
            }
        });
    });
    ui.add_space(8.0);

    if props.scanning {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label("Searching for RadiaCode over USB and Bluetooth…");
        });
        return action;
    }

    if props.devices.is_empty() {
        draw_empty_discovery(ui, props.scanned_once);
        return action;
    }

    egui::ScrollArea::vertical()
        .max_height(240.0)
        .show(ui, |ui| {
            for device in props.devices {
                ui.add_space(4.0);
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(device.display_label()).strong());
                                ui.label(
                                    RichText::new(device.transport_tag())
                                        .small()
                                        .color(MUTED),
                                );
                            });
                            if let Some(serial) = &device.serial {
                                ui.label(RichText::new(serial).small());
                            }
                            ui.label(
                                RichText::new(device.endpoint.address_label())
                                    .monospace()
                                    .small(),
                            );
                            if device.endpoint.transport() == TransportKind::Bluetooth {
                                if let Some(rssi) = device.rssi {
                                    ui.label(RichText::new(format!("{rssi} dBm")).weak().small());
                                }
                            }
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add_enabled(!props.busy, egui::Button::new("Connect"))
                                .clicked()
                            {
                                action = Some(DeviceAction::Connect(device.endpoint.clone()));
                            }
                        });
                    });
                });
            }
        });
    action
}

fn draw_empty_discovery(ui: &mut Ui, scanned_once: bool) {
    if scanned_once {
        ui.label("No detectors found.");
        ui.label(
            RichText::new("Plug in USB, power on Bluetooth, then scan again.")
                .weak()
                .small(),
        );
    } else {
        ui.label(RichText::new("Starting USB and Bluetooth discovery…").weak());
    }
}
