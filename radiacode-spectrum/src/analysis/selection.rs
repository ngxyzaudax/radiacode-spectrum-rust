use crate::analysis::compare::compare;
use crate::analysis::spectrum::CollapsedSpectrum;
use crate::analysis::state::SampleAnalysis;

pub fn rebuild_samples(
    paths_loaded: Vec<Option<CollapsedSpectrum>>,
    background: Option<&CollapsedSpectrum>,
) -> (Vec<SampleAnalysis>, String) {
    let mut samples = Vec::new();
    let mut errors = Vec::new();
    for loaded in paths_loaded {
        let Some(spectrum) = loaded else {
            errors.push("Failed to load a selected sample.".into());
            continue;
        };
        let comparison = match background {
            Some(background) => match compare(&spectrum, background) {
                Ok(comparison) => Some(comparison),
                Err(message) => {
                    errors.push(format!("{}: {message}", spectrum.name));
                    None
                }
            },
            None => None,
        };
        samples.push(SampleAnalysis {
            spectrum,
            comparison,
        });
    }
    (samples, errors.join(" · "))
}

pub fn selection_status(has_background: bool, sample_count: usize) -> String {
    match (has_background, sample_count) {
        (true, 0) => "Background selected. Add one or more samples.".into(),
        (false, 1) => "Sample selected. Add a background to compute net.".into(),
        (false, n) if n > 1 => format!("{n} samples selected. Add a background to compute net."),
        (true, 1) => "Comparison ready.".into(),
        (true, n) if n > 1 => format!("{n} sample comparisons ready."),
        _ => String::new(),
    }
}
