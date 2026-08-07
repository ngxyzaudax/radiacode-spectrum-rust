use egui_plot::Bar;

pub fn outline_points(bars: &[Bar]) -> Vec<[f64; 2]> {
    let mut points = Vec::with_capacity(bars.len().saturating_mul(2));
    for bar in bars {
        let half = bar.bar_width * 0.5;
        points.push([bar.argument - half, bar.value]);
        points.push([bar.argument + half, bar.value]);
    }
    points
}
