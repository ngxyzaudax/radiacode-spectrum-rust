use egui::{Color32, Pos2, Rect, Sense, Stroke, Ui, Vec2};

use crate::model::DeviceInfo;
use crate::theme::{ACCENT, MUTED};

pub fn draw_status_row(ui: &mut Ui, info: &DeviceInfo) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 14.0;
        match info.battery_percent {
            Some(battery) => draw_battery_chip(ui, battery),
            None => draw_battery_pending_chip(ui),
        }
        if let Some(temperature) = info.temperature_c {
            draw_temperature_chip(ui, temperature);
        }
        if let Some(rssi) = info.rssi_dbm {
            draw_signal_chip(ui, rssi);
        }
    });
}

fn draw_battery_pending_chip(ui: &mut Ui) {
    ui.horizontal(|ui| {
        paint_battery_icon(ui, 0.0);
        ui.label(egui::RichText::new("…").strong().color(MUTED));
    });
}

fn draw_battery_chip(ui: &mut Ui, percent: f32) {
    let percent = percent.clamp(0.0, 100.0);
    ui.horizontal(|ui| {
        paint_battery_icon(ui, percent);
        ui.label(
            egui::RichText::new(format!("{percent:.0}%"))
                .strong()
                .color(battery_color(percent)),
        );
    });
}

fn draw_temperature_chip(ui: &mut Ui, celsius: f32) {
    ui.horizontal(|ui| {
        paint_thermometer_icon(ui);
        ui.label(
            egui::RichText::new(format!("{celsius:.1} °C"))
                .strong()
                .color(ACCENT),
        );
    });
}

fn draw_signal_chip(ui: &mut Ui, rssi_dbm: i16) {
    ui.horizontal(|ui| {
        paint_signal_icon(ui, rssi_dbm);
        ui.label(
            egui::RichText::new(format!("{rssi_dbm} dBm"))
                .strong()
                .color(signal_color(rssi_dbm)),
        );
    });
}

fn battery_color(percent: f32) -> Color32 {
    if percent <= 15.0 {
        Color32::from_rgb(220, 90, 90)
    } else if percent <= 35.0 {
        Color32::from_rgb(230, 170, 70)
    } else {
        Color32::from_rgb(110, 190, 120)
    }
}

fn signal_color(rssi_dbm: i16) -> Color32 {
    if rssi_dbm >= -55 {
        Color32::from_rgb(110, 190, 120)
    } else if rssi_dbm >= -75 {
        ACCENT
    } else if rssi_dbm >= -90 {
        Color32::from_rgb(230, 170, 70)
    } else {
        Color32::from_rgb(220, 90, 90)
    }
}

fn signal_bars(rssi_dbm: i16) -> u8 {
    if rssi_dbm >= -55 {
        4
    } else if rssi_dbm >= -65 {
        3
    } else if rssi_dbm >= -75 {
        2
    } else if rssi_dbm >= -85 {
        1
    } else {
        0
    }
}

fn paint_battery_icon(ui: &mut Ui, percent: f32) {
    let size = Vec2::new(22.0, 12.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let painter = ui.painter();
    let body = Rect::from_min_max(
        Pos2::new(rect.left(), rect.top() + 1.0),
        Pos2::new(rect.right() - 3.0, rect.bottom() - 1.0),
    );
    let tip = Rect::from_min_max(
        Pos2::new(body.right(), rect.center().y - 2.5),
        Pos2::new(rect.right(), rect.center().y + 2.5),
    );
    painter.rect_stroke(
        body,
        2.0,
        Stroke::new(1.5, MUTED),
        egui::StrokeKind::Outside,
    );
    painter.rect_filled(tip, 1.0, MUTED);
    let fill_width = ((body.width() - 3.0) * (percent / 100.0)).max(0.0);
    if fill_width > 0.5 {
        let fill = Rect::from_min_size(
            Pos2::new(body.left() + 1.5, body.top() + 1.5),
            Vec2::new(fill_width, body.height() - 3.0),
        );
        painter.rect_filled(fill, 1.0, battery_color(percent));
    }
}

fn paint_thermometer_icon(ui: &mut Ui) {
    let size = Vec2::new(12.0, 18.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let painter = ui.painter();
    let stem = Rect::from_center_size(
        Pos2::new(rect.center().x, rect.top() + 7.0),
        Vec2::new(4.0, 10.0),
    );
    let bulb_center = Pos2::new(rect.center().x, rect.bottom() - 4.0);
    painter.rect_filled(stem, 2.0, MUTED);
    painter.circle_filled(bulb_center, 4.0, ACCENT);
    painter.line_segment(
        [
            Pos2::new(rect.center().x, stem.top() + 1.0),
            Pos2::new(rect.center().x, bulb_center.y),
        ],
        Stroke::new(1.5, ACCENT),
    );
}

fn paint_signal_icon(ui: &mut Ui, rssi_dbm: i16) {
    let size = Vec2::new(18.0, 16.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let painter = ui.painter();
    let active = signal_bars(rssi_dbm);
    let color = signal_color(rssi_dbm);
    let bar_width = 3.0;
    let gap = 2.0;
    let base_y = rect.bottom() - 1.0;
    for index in 0..4 {
        let height = 4.0 + (index as f32) * 3.0;
        let x = rect.left() + (index as f32) * (bar_width + gap);
        let bar = Rect::from_min_max(
            Pos2::new(x, base_y - height),
            Pos2::new(x + bar_width, base_y),
        );
        let fill = index < active as usize;
        if fill {
            painter.rect_filled(bar, 1.0, color);
        } else {
            painter.rect_stroke(
                bar,
                1.0,
                Stroke::new(1.0, MUTED),
                egui::StrokeKind::Outside,
            );
        }
    }
}
