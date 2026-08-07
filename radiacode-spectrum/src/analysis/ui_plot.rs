use egui::RichText;
use egui::Ui;

use crate::analysis::spectrum::CollapsedSpectrum;
use crate::analysis::state::SampleAnalysis;
use crate::analysis::ui_plot_bars::{owned_series, shared_log_floor, show_owned_series};
use crate::analysis::ui_plot_legend::draw_legend;
use crate::analysis::ui_plot_values::{smoothed_background, smoothed_net, smoothed_sample};
use crate::scale::{HistogramStyle, YScale};
use crate::theme::{analysis_sample_color, ANALYSIS_BACKGROUND, MUTED};

pub fn draw_analysis_plots(
    ui: &mut Ui,
    samples: &[SampleAnalysis],
    background: Option<&CollapsedSpectrum>,
    y_scale: YScale,
    smooth_window: usize,
    style: HistogramStyle,
) {
    if samples.is_empty() && background.is_none() {
        return;
    }
    let Some(axis) = samples.first().map(|s| &s.spectrum).or(background) else {
        return;
    };
    let energies = &axis.energies_kev;
    let width = axis.a1 as f64;
    draw_legend(ui, samples, background.is_some());
    ui.label("Overlay");
    draw_overlay_plot(
        ui,
        energies,
        width,
        samples,
        background,
        y_scale,
        smooth_window,
        style,
    );
    ui.add_space(12.0);
    ui.label("Net (sample − background)");
    draw_net_plot(
        ui,
        energies,
        width,
        samples,
        background,
        y_scale,
        smooth_window,
        style,
    );
}

fn draw_overlay_plot(
    ui: &mut Ui,
    energies: &[f64],
    width: f64,
    samples: &[SampleAnalysis],
    background: Option<&CollapsedSpectrum>,
    y_scale: YScale,
    smooth_window: usize,
    style: HistogramStyle,
) {
    let background_values = background.map(|item| smoothed_background(item, smooth_window));
    let sample_values: Vec<_> = samples
        .iter()
        .map(|sample| smoothed_sample(sample, smooth_window))
        .collect();
    let mut rate_sets: Vec<&[f64]> = background_values
        .as_ref()
        .map(|values| vec![values.as_slice()])
        .unwrap_or_default();
    rate_sets.extend(sample_values.iter().map(|values| values.as_slice()));
    let log_floor = shared_log_floor(&rate_sets);
    let mut owned = Vec::new();
    if let Some(values) = background_values.as_ref() {
        owned.push(owned_series(
            "background",
            energies,
            width,
            values,
            ANALYSIS_BACKGROUND,
            y_scale,
            log_floor,
        ));
    }
    for (index, values) in sample_values.iter().enumerate() {
        owned.push(owned_series(
            &format!("sample_{index}"),
            energies,
            width,
            values,
            analysis_sample_color(index),
            y_scale,
            log_floor,
        ));
    }
    show_owned_series(ui, "analysis_overlay", &owned, y_scale, style);
}

fn draw_net_plot(
    ui: &mut Ui,
    energies: &[f64],
    width: f64,
    samples: &[SampleAnalysis],
    background: Option<&CollapsedSpectrum>,
    y_scale: YScale,
    smooth_window: usize,
    style: HistogramStyle,
) {
    let Some(background) = background else {
        ui.label(RichText::new("Select a background recording to compute net spectra.").color(MUTED));
        return;
    };
    if samples.is_empty() {
        ui.label(RichText::new("Add one or more sample recordings.").color(MUTED));
        return;
    }
    let net_values: Vec<_> = samples
        .iter()
        .filter_map(|sample| {
            sample
                .comparison
                .as_ref()
                .map(|comparison| smoothed_net(sample, comparison, background, smooth_window))
        })
        .collect();
    if net_values.is_empty() {
        ui.label(RichText::new("No comparable samples for net plot.").color(MUTED));
        return;
    }
    let rate_sets: Vec<&[f64]> = net_values.iter().map(|values| values.as_slice()).collect();
    let log_floor = shared_log_floor(&rate_sets);
    let owned: Vec<_> = net_values
        .iter()
        .enumerate()
        .map(|(index, values)| {
            owned_series(
                &format!("net_{index}"),
                energies,
                width,
                values,
                analysis_sample_color(index),
                y_scale,
                log_floor,
            )
        })
        .collect();
    show_owned_series(ui, "analysis_net", &owned, y_scale, style);
}
