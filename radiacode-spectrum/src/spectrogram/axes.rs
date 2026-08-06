use egui::{Align2, FontId, Ui};

use crate::theme::MUTED;
use crate::spectrogram::model::SpectrogramSeries;

pub fn draw_header_line(
    ui: &mut Ui,
    series: &SpectrogramSeries,
    viewing_library: bool,
) {
    let duration = format_duration(series.duration_secs());
    let interval = series.header.interval_secs.round() as u64;
    let (gap_count, gap_offline) = series.gap_summary();
    let gap_suffix = if gap_count > 0 {
        format!("  |  gaps: {gap_count} ({})", format_duration(gap_offline))
    } else {
        String::new()
    };
    let mode = if viewing_library { "library" } else { "live" };
    let energy_max = series
        .energies_kev
        .last()
        .copied()
        .unwrap_or(0.0);
    ui.label(
        egui::RichText::new(format!(
            "{mode}  |  total {duration}  |  interval {interval}s  |  {} ch  |  0–{energy_max:.0} keV{gap_suffix}",
            series.header.channel_count
        ))
        .small()
        .color(MUTED),
    );
}

pub fn draw_x_axis(
    painter: &egui::Painter,
    ui: &Ui,
    image_rect: egui::Rect,
    series: &SpectrogramSeries,
    source_cols: &[usize],
) {
    let _ = ui;
    if source_cols.is_empty() {
        return;
    }
    let font = FontId::new(11.0, egui::FontFamily::Proportional);
    for step in 0..=4 {
        let t = step as f32 / 4.0;
        let x = egui::lerp(image_rect.left()..=image_rect.right(), t);
        let index = ((t * (source_cols.len().saturating_sub(1)) as f32).round() as usize)
            .min(source_cols.len().saturating_sub(1));
        let channel = source_cols[index];
        let energy = series.energies_kev.get(channel).copied().unwrap_or(0.0);
        painter.text(
            egui::pos2(x, image_rect.bottom() + 2.0),
            Align2::CENTER_TOP,
            format!("{energy:.0} keV"),
            font.clone(),
            MUTED,
        );
    }
}

pub fn y_axis_label() -> &'static str {
    "Time"
}

pub fn x_axis_label() -> &'static str {
    "Energy (keV)"
}

fn format_duration(secs: f64) -> String {
    let total = secs.max(0.0).round() as u64;
    let hours = total / 3600;
    let minutes = (total % 3600) / 60;
    let seconds = total % 60;
    format!("{hours:02}:{minutes:02}:{seconds:02}")
}
