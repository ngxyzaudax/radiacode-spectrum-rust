use crate::spectrogram::colormap::percentile_peak;
use crate::spectrogram::settings::SpectrogramSettings;

pub struct ZScaleRange {
    pub min: f32,
    pub max: f32,
}

pub fn resolve_z_range(settings: &SpectrogramSettings, values: &[u32]) -> ZScaleRange {
    if settings.auto_brightness && !values.is_empty() {
        let peak = percentile_peak(values, 0.98).max(1.0);
        return ZScaleRange {
            min: 0.0,
            max: peak,
        };
    }
    ZScaleRange {
        min: settings.z_min,
        max: settings.z_max.max(settings.z_min + 1.0),
    }
}

pub fn map_count(value: f32, range: &ZScaleRange) -> f32 {
    if value <= 0.0 {
        return 0.0;
    }
    let span = (range.max - range.min).max(1.0);
    ((value - range.min) / span).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::{map_count, resolve_z_range, ZScaleRange};
    use crate::spectrogram::settings::SpectrogramSettings;

    #[test]
    fn auto_brightness_uses_peak() {
        let mut settings = SpectrogramSettings::default();
        settings.auto_brightness = true;
        let range = resolve_z_range(&settings, &[1, 2, 100]);
        assert!(range.max >= 100.0);
    }

    #[test]
    fn linear_mapping_is_monotonic() {
        let range = ZScaleRange {
            min: 0.0,
            max: 1000.0,
        };
        let low = map_count(10.0, &range);
        let high = map_count(100.0, &range);
        assert!(high > low);
    }
}
