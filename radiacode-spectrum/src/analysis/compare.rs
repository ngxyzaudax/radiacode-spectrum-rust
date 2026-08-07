use crate::analysis::spectrum::CollapsedSpectrum;

const CALIB_EPS: f32 = 1e-4;

#[derive(Debug, Clone, PartialEq)]
pub struct Comparison {
    pub scale_factor: f64,
    pub scaled_background_counts: Vec<f64>,
    pub net_counts: Vec<f64>,
    pub calib_warning: bool,
    pub negative_bin_count: usize,
    pub net_total: f64,
    pub net_min: f64,
}

pub fn compare(
    sample: &CollapsedSpectrum,
    background: &CollapsedSpectrum,
) -> Result<Comparison, String> {
    if sample.channel_count != background.channel_count {
        return Err(format!(
            "channel count mismatch: sample {} vs background {}",
            sample.channel_count, background.channel_count
        ));
    }
    if sample.live_time_secs <= 0.0 {
        return Err("sample live time must be positive".into());
    }
    if background.live_time_secs <= 0.0 {
        return Err("background live time must be positive".into());
    }
    let scale_factor = sample.live_time_secs / background.live_time_secs;
    let calib_warning = calibrations_differ(sample, background);
    let mut scaled_background_counts = Vec::with_capacity(sample.counts.len());
    let mut net_counts = Vec::with_capacity(sample.counts.len());
    let mut negative_bin_count = 0usize;
    let mut net_total = 0.0;
    let mut net_min = f64::MAX;
    for (sample_count, background_count) in sample.counts.iter().zip(background.counts.iter()) {
        let scaled = *background_count as f64 * scale_factor;
        let net = *sample_count as f64 - scaled;
        if net < 0.0 {
            negative_bin_count += 1;
        }
        net_total += net;
        net_min = net_min.min(net);
        scaled_background_counts.push(scaled);
        net_counts.push(net);
    }
    if net_min == f64::MAX {
        net_min = 0.0;
    }
    Ok(Comparison {
        scale_factor,
        scaled_background_counts,
        net_counts,
        calib_warning,
        negative_bin_count,
        net_total,
        net_min,
    })
}

fn calibrations_differ(sample: &CollapsedSpectrum, background: &CollapsedSpectrum) -> bool {
    (sample.a0 - background.a0).abs() > CALIB_EPS
        || (sample.a1 - background.a1).abs() > CALIB_EPS
        || (sample.a2 - background.a2).abs() > CALIB_EPS
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::compare;
    use crate::analysis::spectrum::CollapsedSpectrum;

    fn fixture(counts: Vec<u64>, live_time_secs: f64) -> CollapsedSpectrum {
        let total_counts: u64 = counts.iter().sum();
        let channel_count = counts.len() as u32;
        let energies_kev: Vec<f64> = (0..counts.len()).map(|i| i as f64).collect();
        CollapsedSpectrum {
            name: "x".into(),
            path: PathBuf::from("/tmp/x.rcwf"),
            a0: 0.0,
            a1: 1.0,
            a2: 0.0,
            channel_count,
            energies_kev,
            counts,
            live_time_secs,
            total_counts,
            gap_count: 0,
            gap_offline_secs: 0.0,
            device_serial: None,
        }
    }

    #[test]
    fn subtract_scales_background_by_live_time_ratio() {
        let sample = fixture(vec![100, 200], 20.0);
        let background = fixture(vec![10, 20], 10.0);
        let comparison = compare(&sample, &background).expect("compare");
        assert!((comparison.scale_factor - 2.0).abs() < 0.001);
        assert!((comparison.scaled_background_counts[0] - 20.0).abs() < 0.001);
        assert!((comparison.net_counts[0] - 80.0).abs() < 0.001);
        assert!((comparison.net_counts[1] - 160.0).abs() < 0.001);
        assert_eq!(comparison.negative_bin_count, 0);
    }

    #[test]
    fn rejects_zero_live_time() {
        let sample = fixture(vec![1], 0.0);
        let background = fixture(vec![1], 10.0);
        assert!(compare(&sample, &background).is_err());
    }

    #[test]
    fn rejects_channel_mismatch() {
        let sample = fixture(vec![1, 2], 10.0);
        let background = fixture(vec![1], 10.0);
        assert!(compare(&sample, &background).is_err());
    }
}
