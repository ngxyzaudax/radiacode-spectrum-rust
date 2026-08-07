use crate::analysis::compare::Comparison;
use crate::analysis::spectrum::{counts_per_sec, CollapsedSpectrum};
use crate::analysis::state::SampleAnalysis;
use crate::smooth::moving_average_f64;

pub fn smoothed_cps(counts: &[u64], live_time_secs: f64, smooth_window: usize) -> Vec<f64> {
    moving_average_f64(&counts_per_sec(counts, live_time_secs), smooth_window)
}

pub fn smoothed_sample(sample: &SampleAnalysis, smooth_window: usize) -> Vec<f64> {
    smoothed_cps(
        &sample.spectrum.counts,
        sample.spectrum.live_time_secs,
        smooth_window,
    )
}

pub fn smoothed_background(background: &CollapsedSpectrum, smooth_window: usize) -> Vec<f64> {
    smoothed_cps(&background.counts, background.live_time_secs, smooth_window)
}

pub fn smoothed_net(
    sample: &SampleAnalysis,
    _comparison: &Comparison,
    background: &CollapsedSpectrum,
    smooth_window: usize,
) -> Vec<f64> {
    let sample_values = smoothed_sample(sample, smooth_window);
    let background_values = smoothed_background(background, smooth_window);
    sample_values
        .iter()
        .zip(background_values.iter())
        .map(|(sample_value, background_value)| (sample_value - background_value).max(0.0))
        .collect()
}
