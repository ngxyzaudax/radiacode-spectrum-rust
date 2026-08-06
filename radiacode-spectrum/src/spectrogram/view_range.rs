use crate::energy::{ENERGY_MAX_KEV, ENERGY_MIN_KEV};

const FIT_FULL_THRESHOLD: f64 = 0.98;

#[derive(Debug, Clone, Copy)]
pub struct SpectrogramViewRange {
    pub energy_min_kev: f64,
    pub energy_max_kev: f64,
    pub series_energy_min_kev: f64,
    pub series_energy_max_kev: f64,
    pub channel_start: usize,
    pub row_start: usize,
    pub follow_live: bool,
    pub fit_full_spectrum: bool,
}

impl SpectrogramViewRange {
    pub fn new() -> Self {
        Self {
            energy_min_kev: ENERGY_MIN_KEV,
            energy_max_kev: ENERGY_MAX_KEV,
            series_energy_min_kev: ENERGY_MIN_KEV,
            series_energy_max_kev: ENERGY_MAX_KEV,
            channel_start: 0,
            row_start: 0,
            follow_live: true,
            fit_full_spectrum: true,
        }
    }

    pub fn fit_series_energy(&mut self, energies_kev: &[f64]) {
        if energies_kev.is_empty() {
            self.reset();
            return;
        }
        let min = energies_kev
            .first()
            .copied()
            .unwrap_or(ENERGY_MIN_KEV)
            .max(ENERGY_MIN_KEV);
        let max = energies_kev
            .last()
            .copied()
            .unwrap_or(ENERGY_MAX_KEV)
            .min(ENERGY_MAX_KEV);
        self.series_energy_min_kev = min;
        self.series_energy_max_kev = max;
        self.energy_min_kev = min;
        self.energy_max_kev = max;
        self.channel_start = 0;
        self.row_start = 0;
        self.follow_live = true;
        self.fit_full_spectrum = true;
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    pub fn full_series_span(&self) -> f64 {
        (self.series_energy_max_kev - self.series_energy_min_kev).max(1.0)
    }

    pub fn zoom_energy(&mut self, anchor_kev: f64, factor: f64) {
        let bounds_min = self.series_energy_min_kev;
        let bounds_max = self.series_energy_max_kev;
        let full_span = self.full_series_span();
        let span = (self.energy_max_kev - self.energy_min_kev).max(1.0);
        let new_span = (span * factor).clamp(1.0, full_span);
        if new_span >= full_span * FIT_FULL_THRESHOLD {
            self.snap_full_spectrum();
            return;
        }
        self.fit_full_spectrum = false;
        let ratio = ((anchor_kev - self.energy_min_kev) / span).clamp(0.0, 1.0);
        let mut min = anchor_kev - new_span * ratio;
        let mut max = min + new_span;
        if min < bounds_min {
            min = bounds_min;
            max = min + new_span;
        }
        if max > bounds_max {
            max = bounds_max;
            min = max - new_span;
        }
        min = min.max(bounds_min);
        max = max.min(bounds_max).max(min + 1.0);
        self.energy_min_kev = min;
        self.energy_max_kev = max;
        self.channel_start = 0;
    }

    pub fn pan_energy(&mut self, delta_kev: f64) {
        if delta_kev.abs() > 0.0 {
            self.fit_full_spectrum = false;
        }
        let bounds_min = self.series_energy_min_kev;
        let bounds_max = self.series_energy_max_kev;
        let span = self.energy_max_kev - self.energy_min_kev;
        let mut min = self.energy_min_kev + delta_kev;
        let mut max = min + span;
        if min < bounds_min {
            min = bounds_min;
            max = min + span;
        }
        if max > bounds_max {
            max = bounds_max;
            min = max - span;
        }
        self.energy_min_kev = min.max(bounds_min);
        self.energy_max_kev = max.min(bounds_max);
        self.channel_start = 0;
    }

    fn snap_full_spectrum(&mut self) {
        self.energy_min_kev = self.series_energy_min_kev;
        self.energy_max_kev = self.series_energy_max_kev;
        self.channel_start = 0;
        self.fit_full_spectrum = true;
    }

    pub fn scroll_channels(&mut self, delta: i32, channels_in_view: usize, display_cols: usize) {
        if delta != 0 {
            self.fit_full_spectrum = false;
        }
        if channels_in_view <= display_cols {
            self.channel_start = 0;
            return;
        }
        let max_start = channels_in_view - display_cols;
        let next = (self.channel_start as i32 + delta).clamp(0, max_start as i32) as usize;
        self.channel_start = next;
    }

    pub fn clamp_channels(&mut self, channels_in_view: usize, display_cols: usize) {
        if self.fit_full_spectrum {
            self.channel_start = 0;
            return;
        }
        if channels_in_view <= display_cols {
            self.channel_start = 0;
            return;
        }
        let max_start = channels_in_view - display_cols;
        self.channel_start = self.channel_start.min(max_start);
    }

    pub fn visible_start(&self, total_rows: usize, visible_rows: usize) -> usize {
        if self.follow_live || total_rows <= visible_rows {
            return total_rows.saturating_sub(visible_rows);
        }
        let max_start = total_rows.saturating_sub(visible_rows);
        self.row_start.min(max_start)
    }

    pub fn scroll_history(&mut self, row_delta: i32, total_rows: usize, visible_rows: usize) {
        if total_rows <= visible_rows {
            self.follow_live = true;
            self.row_start = 0;
            return;
        }
        let max_start = total_rows - visible_rows;
        let current = self.visible_start(total_rows, visible_rows);
        let next = (current as i32 + row_delta).clamp(0, max_start as i32) as usize;
        self.row_start = next;
        self.follow_live = next >= max_start;
    }

    pub fn clamp_to_history(&mut self, total_rows: usize, visible_rows: usize) {
        if self.follow_live || total_rows <= visible_rows {
            self.follow_live = true;
            self.row_start = total_rows.saturating_sub(visible_rows);
            return;
        }
        let max_start = total_rows.saturating_sub(visible_rows);
        self.row_start = self.row_start.min(max_start);
    }
}

#[cfg(test)]
mod tests {
    use super::SpectrogramViewRange;

    fn sample_energies() -> Vec<f64> {
        (0..512).map(|ch| ch as f64 * (2804.0 / 511.0)).collect()
    }

    #[test]
    fn fit_series_uses_calibrated_bounds() {
        let mut range = SpectrogramViewRange::new();
        let energies = sample_energies();
        range.fit_series_energy(&energies);
        assert!((range.energy_max_kev - 2804.0).abs() < 1.0);
        assert!(range.fit_full_spectrum);
    }

    #[test]
    fn zoom_out_restores_full_spectrum() {
        let mut range = SpectrogramViewRange::new();
        let energies = sample_energies();
        range.fit_series_energy(&energies);
        range.zoom_energy(500.0, 0.2);
        assert!(!range.fit_full_spectrum);
        range.zoom_energy(500.0, 6.0);
        assert!(range.fit_full_spectrum);
        assert!((range.energy_max_kev - 2804.0).abs() < 1.0);
    }
}
