use egui::{Color32, CornerRadius, Stroke, Visuals};

pub fn apply(ctx: &egui::Context) {
    let mut visuals = Visuals::dark();
    visuals.window_fill = Color32::from_rgb(18, 20, 24);
    visuals.panel_fill = Color32::from_rgb(24, 27, 33);
    visuals.extreme_bg_color = Color32::from_rgb(12, 14, 18);
    visuals.faint_bg_color = Color32::from_rgb(32, 36, 44);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, Color32::from_rgb(210, 214, 222));
    visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(28, 32, 40);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, Color32::from_rgb(220, 224, 232));
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(36, 40, 48);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, Color32::from_rgb(245, 248, 252));
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(48, 54, 64);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, Color32::from_rgb(255, 255, 255));
    visuals.widgets.active.bg_fill = Color32::from_rgb(72, 132, 196);
    visuals.selection.bg_fill = Color32::from_rgb(72, 132, 196);
    visuals.selection.stroke = Stroke::new(1.0, Color32::from_rgb(120, 168, 220));
    visuals.override_text_color = Some(Color32::from_rgb(230, 234, 240));
    visuals.window_corner_radius = CornerRadius::same(6);
    visuals.menu_corner_radius = CornerRadius::same(4);
    ctx.set_visuals(visuals);
}

pub const SPECTRUM_BAR: Color32 = Color32::from_rgb(245, 196, 0);
pub const ACCENT: Color32 = Color32::from_rgb(120, 168, 220);
pub const MUTED: Color32 = Color32::from_rgb(150, 158, 172);
pub const ANALYSIS_BACKGROUND: Color32 = ACCENT;

const ANALYSIS_SAMPLE_COLORS: [Color32; 8] = [
    Color32::from_rgb(245, 196, 0),
    Color32::from_rgb(236, 120, 96),
    Color32::from_rgb(168, 132, 232),
    Color32::from_rgb(80, 196, 196),
    Color32::from_rgb(240, 152, 64),
    Color32::from_rgb(232, 120, 180),
    Color32::from_rgb(140, 208, 96),
    Color32::from_rgb(96, 168, 240),
];

pub fn analysis_sample_color(index: usize) -> Color32 {
    ANALYSIS_SAMPLE_COLORS[index % ANALYSIS_SAMPLE_COLORS.len()]
}
