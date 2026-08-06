use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpectrogramHeader {
    pub created_at: String,
    pub a0: f32,
    pub a1: f32,
    pub a2: f32,
    pub channel_count: u32,
    pub interval_secs: f64,
    pub device_serial: Option<String>,
    pub energies_kev: Vec<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RowKind {
    Normal,
    GapRecovery { offline_secs: f64, raw_total: u64 },
    LiveSpike { rate_factor: f32 },
}

impl RowKind {
    pub fn storage_tag(self) -> u8 {
        match self {
            Self::Normal => 0,
            Self::GapRecovery { .. } => 1,
            Self::LiveSpike { .. } => 2,
        }
    }

    pub fn from_storage_tag(tag: u8, extra: f64, raw_total: u64) -> Self {
        match tag {
            1 => Self::GapRecovery {
                offline_secs: extra,
                raw_total,
            },
            2 => Self::LiveSpike {
                rate_factor: extra as f32,
            },
            _ => Self::Normal,
        }
    }

    pub fn storage_extra(self) -> f64 {
        match self {
            Self::GapRecovery { offline_secs, .. } => offline_secs,
            Self::LiveSpike { rate_factor } => f64::from(rate_factor),
            Self::Normal => 0.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SpectrogramRow {
    pub elapsed_secs: f64,
    pub interval_secs: f64,
    pub kind: RowKind,
    pub counts: Vec<u32>,
}

impl SpectrogramRow {
    pub fn row_total(&self) -> u64 {
        self.counts.iter().map(|&value| value as u64).sum()
    }
}

#[derive(Debug, Clone)]
pub struct SpectrogramSeries {
    pub header: SpectrogramHeader,
    pub energies_kev: Vec<f64>,
    pub rows: Vec<SpectrogramRow>,
}

impl SpectrogramSeries {
    pub fn new(header: SpectrogramHeader, energies_kev: Vec<f64>) -> Self {
        Self {
            header,
            energies_kev,
            rows: Vec::new(),
        }
    }

    pub fn push_row(
        &mut self,
        counts: Vec<u32>,
        interval_secs: f64,
        kind: RowKind,
        max_samples: usize,
    ) {
        let elapsed_secs = if let Some(last) = self.rows.last() {
            last.elapsed_secs + last.interval_secs
        } else {
            0.0
        };
        self.rows.push(SpectrogramRow {
            elapsed_secs,
            interval_secs,
            kind,
            counts,
        });
        let cap = max_samples.max(100);
        if self.rows.len() > cap {
            let drop = self.rows.len() - cap;
            self.rows.drain(0..drop);
        }
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn duration_secs(&self) -> f64 {
        self.rows
            .iter()
            .map(|row| row.interval_secs)
            .sum()
    }

    pub fn gap_summary(&self) -> (usize, f64) {
        let mut count = 0usize;
        let mut offline_secs = 0.0;
        for row in &self.rows {
            if let RowKind::GapRecovery { offline_secs: gap, .. } = row.kind {
                count += 1;
                offline_secs += gap;
            }
        }
        (count, offline_secs)
    }

    pub fn recent_row_totals(&self, limit: usize) -> Vec<u64> {
        self.rows
            .iter()
            .rev()
            .filter(|row| matches!(row.kind, RowKind::Normal | RowKind::LiveSpike { .. }))
            .take(limit)
            .map(|row| row.row_total())
            .collect()
    }

    pub fn age_secs_before(&self, row_index: usize) -> f64 {
        self.rows
            .iter()
            .skip(row_index + 1)
            .map(|row| row.interval_secs)
            .sum()
    }

    pub fn row_window(&self, start: usize, visible_rows: usize) -> &[SpectrogramRow] {
        if self.rows.is_empty() || visible_rows == 0 {
            return &[];
        }
        let start = start.min(self.rows.len().saturating_sub(1));
        let end = (start + visible_rows).min(self.rows.len());
        &self.rows[start..end]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpectrogramDisplay {
    Live,
    Loaded,
}

#[derive(Debug, Clone)]
pub struct RecordingEntry {
    pub path: PathBuf,
    pub name: String,
    pub comment: String,
    pub created_at: String,
    pub device_serial: Option<String>,
    pub interval_secs: f64,
    pub row_count: u32,
    pub channel_count: u32,
}

#[cfg(test)]
mod tests {
    use super::{RowKind, SpectrogramHeader, SpectrogramSeries};

    fn sample_header() -> SpectrogramHeader {
        SpectrogramHeader {
            created_at: "t".into(),
            a0: 0.0,
            a1: 1.0,
            a2: 0.0,
            channel_count: 2,
            interval_secs: 5.0,
            device_serial: None,
            energies_kev: vec![100.0, 200.0],
        }
    }

    #[test]
    fn duration_secs_sums_variable_intervals() {
        let mut series = SpectrogramSeries::new(sample_header(), vec![100.0, 200.0]);
        series.push_row(vec![1, 2], 5.0, RowKind::Normal, 100);
        series.push_row(vec![3, 4], 45.0, RowKind::GapRecovery { offline_secs: 45.0, raw_total: 7 }, 100);
        series.push_row(vec![5, 6], 5.0, RowKind::Normal, 100);
        assert!((series.duration_secs() - 55.0).abs() < 0.001);
    }

    #[test]
    fn age_secs_before_uses_row_intervals() {
        let mut series = SpectrogramSeries::new(sample_header(), vec![100.0, 200.0]);
        series.push_row(vec![1, 2], 5.0, RowKind::Normal, 100);
        series.push_row(vec![3, 4], 45.0, RowKind::GapRecovery { offline_secs: 45.0, raw_total: 7 }, 100);
        series.push_row(vec![5, 6], 5.0, RowKind::Normal, 100);
        assert!((series.age_secs_before(0) - 50.0).abs() < 0.001);
    }
}
