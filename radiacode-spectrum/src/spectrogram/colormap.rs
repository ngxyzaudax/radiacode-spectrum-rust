use egui::Color32;

use crate::spectrogram::color_scheme::ColorScheme;

pub fn count_to_color(value: f32, max_value: f32, scheme: ColorScheme) -> Color32 {
    if max_value <= 0.0 || value <= 0.0 {
        return Color32::from_rgb(8, 10, 16);
    }
    let t = (value / max_value).clamp(0.0, 1.0);
    color_from_t(t, scheme)
}

pub fn normalized_to_color(t: f32, scheme: ColorScheme) -> Color32 {
    color_from_t(t.clamp(0.0, 1.0), scheme)
}

fn color_from_t(t: f32, scheme: ColorScheme) -> Color32 {
    match scheme {
        ColorScheme::Viridis => viridis_color(t),
        ColorScheme::Inferno => inferno_color(t),
        ColorScheme::Turbo => turbo_color(t),
    }
}

fn viridis_color(t: f32) -> Color32 {
    if t < 0.25 {
        lerp_rgb([68, 1, 84], [59, 82, 139], t / 0.25)
    } else if t < 0.5 {
        lerp_rgb([59, 82, 139], [33, 145, 140], (t - 0.25) / 0.25)
    } else if t < 0.75 {
        lerp_rgb([33, 145, 140], [94, 201, 98], (t - 0.5) / 0.25)
    } else {
        lerp_rgb([94, 201, 98], [253, 231, 37], (t - 0.75) / 0.25)
    }
}

fn inferno_color(t: f32) -> Color32 {
    if t < 0.25 {
        lerp_rgb([0, 0, 4], [87, 16, 110], t / 0.25)
    } else if t < 0.5 {
        lerp_rgb([87, 16, 110], [188, 55, 84], (t - 0.25) / 0.25)
    } else if t < 0.75 {
        lerp_rgb([188, 55, 84], [249, 142, 9], (t - 0.5) / 0.25)
    } else {
        lerp_rgb([249, 142, 9], [252, 255, 164], (t - 0.75) / 0.25)
    }
}

fn turbo_color(t: f32) -> Color32 {
    if t < 0.25 {
        lerp_rgb([48, 18, 59], [33, 144, 255], t / 0.25)
    } else if t < 0.5 {
        lerp_rgb([33, 144, 255], [0, 220, 130], (t - 0.25) / 0.25)
    } else if t < 0.75 {
        lerp_rgb([0, 220, 130], [255, 210, 0], (t - 0.5) / 0.25)
    } else {
        lerp_rgb([255, 210, 0], [220, 40, 20], (t - 0.75) / 0.25)
    }
}

fn lerp_rgb(from: [u8; 3], to: [u8; 3], t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgb(
        lerp_u8(from[0], to[0], t),
        lerp_u8(from[1], to[1], t),
        lerp_u8(from[2], to[2], t),
    )
}

fn lerp_u8(from: u8, to: u8, t: f32) -> u8 {
    (from as f32 + (to as f32 - from as f32) * t).round() as u8
}

pub fn percentile_peak(values: &[u32], percentile: f32) -> f32 {
    let mut sorted: Vec<u32> = values.iter().copied().filter(|&value| value > 0).collect();
    if sorted.is_empty() {
        return 1.0;
    }
    sorted.sort_unstable();
    let index = ((sorted.len() as f32 - 1.0) * percentile.clamp(0.0, 1.0)).round() as usize;
    sorted[index.min(sorted.len() - 1)].max(1) as f32
}

#[cfg(test)]
mod tests {
    use super::{count_to_color, percentile_peak};
    use crate::spectrogram::color_scheme::ColorScheme;

    #[test]
    fn zero_is_dark() {
        assert_eq!(
            count_to_color(0.0, 10.0, ColorScheme::Viridis),
            egui::Color32::from_rgb(8, 10, 16)
        );
    }

    #[test]
    fn peak_percentile_picks_high_bin() {
        assert!(percentile_peak(&[1, 2, 100, 3], 0.98) >= 100.0);
    }

    #[test]
    fn zeros_do_not_collapse_auto_peak() {
        let mut values = vec![0u32; 200];
        values[10] = 1;
        values[11] = 4;
        values[12] = 12;
        assert!(percentile_peak(&values, 0.98) >= 4.0);
    }
}
