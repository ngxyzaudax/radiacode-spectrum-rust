use std::path::PathBuf;

use crate::spectrogram::model::{RecordingEntry, SpectrogramSeries};

#[derive(Debug, Clone, PartialEq)]
pub struct CollapsedSpectrum {
    pub name: String,
    pub path: PathBuf,
    pub a0: f32,
    pub a1: f32,
    pub a2: f32,
    pub channel_count: u32,
    pub energies_kev: Vec<f64>,
    pub counts: Vec<u64>,
    pub live_time_secs: f64,
    pub total_counts: u64,
    pub gap_count: usize,
    pub gap_offline_secs: f64,
    pub device_serial: Option<String>,
}

pub fn collapse_series(series: &SpectrogramSeries, entry: &RecordingEntry) -> CollapsedSpectrum {
    let channel_count = series.header.channel_count as usize;
    let mut counts = vec![0_u64; channel_count];
    for row in &series.rows {
        for (index, value) in row.counts.iter().enumerate().take(channel_count) {
            counts[index] += u64::from(*value);
        }
    }
    let (gap_count, gap_offline_secs) = series.gap_summary();
    let total_counts = counts.iter().sum();
    CollapsedSpectrum {
        name: entry.name.clone(),
        path: entry.path.clone(),
        a0: series.header.a0,
        a1: series.header.a1,
        a2: series.header.a2,
        channel_count: series.header.channel_count,
        energies_kev: series.energies_kev.clone(),
        counts,
        live_time_secs: series.duration_secs(),
        total_counts,
        gap_count,
        gap_offline_secs,
        device_serial: series.header.device_serial.clone(),
    }
}

pub fn counts_per_sec(values: &[u64], live_time_secs: f64) -> Vec<f64> {
    if live_time_secs <= 0.0 {
        return values.iter().map(|_| 0.0).collect();
    }
    values
        .iter()
        .map(|value| *value as f64 / live_time_secs)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{collapse_series, counts_per_sec};
    use crate::spectrogram::model::{
        RecordingEntry, RowKind, SpectrogramHeader, SpectrogramRow, SpectrogramSeries,
    };

    fn sample_entry() -> RecordingEntry {
        RecordingEntry {
            path: "/tmp/test.rcwf".into(),
            name: "test".into(),
            comment: String::new(),
            created_at: String::new(),
            device_serial: Some("RC-110".into()),
            interval_secs: 1.0,
            row_count: 2,
            channel_count: 2,
        }
    }

    fn sample_header() -> SpectrogramHeader {
        SpectrogramHeader {
            created_at: "t".into(),
            a0: 0.0,
            a1: 1.0,
            a2: 0.0,
            channel_count: 2,
            interval_secs: 1.0,
            device_serial: Some("RC-110".into()),
            energies_kev: vec![0.0, 1.0],
        }
    }

    #[test]
    fn collapse_sums_rows_and_includes_gap_live_time() {
        let mut series = SpectrogramSeries::new(sample_header(), vec![0.0, 1.0]);
        series.rows.push(SpectrogramRow {
            elapsed_secs: 0.0,
            interval_secs: 1.0,
            kind: RowKind::Normal,
            counts: vec![10, 20],
        });
        series.rows.push(SpectrogramRow {
            elapsed_secs: 1.0,
            interval_secs: 5.0,
            kind: RowKind::GapRecovery {
                offline_secs: 5.0,
                raw_total: 15,
            },
            counts: vec![5, 10],
        });
        let collapsed = collapse_series(&series, &sample_entry());
        assert_eq!(collapsed.counts, vec![15, 30]);
        assert_eq!(collapsed.total_counts, 45);
        assert!((collapsed.live_time_secs - 6.0).abs() < 0.001);
        assert_eq!(collapsed.gap_count, 1);
        assert!((collapsed.gap_offline_secs - 5.0).abs() < 0.001);
    }

    #[test]
    fn counts_per_sec_divides_by_live_time() {
        let cps = counts_per_sec(&[100, 200], 10.0);
        assert!((cps[0] - 10.0).abs() < 0.001);
        assert!((cps[1] - 20.0).abs() < 0.001);
    }
}
