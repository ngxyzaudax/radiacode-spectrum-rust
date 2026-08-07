use egui::{RichText, Ui};

use crate::analysis::state::AnalysisState;
use crate::analysis::ui_plot::draw_analysis_plots;
use crate::scale::{HistogramStyle, YScale};
use crate::theme::{analysis_sample_color, MUTED};

pub fn draw_analysis_view(ui: &mut Ui, state: &AnalysisState, y_scale: YScale) {
    if state.samples.is_empty() && state.background.is_none() {
        ui.add_space(12.0);
        ui.label(
            RichText::new(
                "Select one background and one or more sample recordings from the library sidebar.",
            )
            .color(MUTED),
        );
        return;
    }
    let style = if state.outline_only {
        HistogramStyle::Outline
    } else {
        HistogramStyle::Filled
    };
    draw_analysis_plots(
        ui,
        &state.samples,
        state.background.as_ref(),
        y_scale,
        state.smooth_window,
        style,
    );
    draw_footer_readouts(ui, state);
}

fn draw_footer_readouts(ui: &mut Ui, state: &AnalysisState) {
    let comparable: Vec<_> = state
        .samples
        .iter()
        .enumerate()
        .filter_map(|(index, sample)| {
            sample
                .comparison
                .as_ref()
                .map(|comparison| (index, sample, comparison))
        })
        .collect();
    if comparable.is_empty() {
        return;
    }
    ui.add_space(12.0);
    ui.separator();
    for (index, sample, comparison) in comparable {
        let color = analysis_sample_color(index);
        let live = sample.spectrum.live_time_secs;
        let net_cps = comparison.net_total / live;
        let net_min_cps = comparison.net_min / live;
        ui.horizontal(|ui| {
            ui.label(RichText::new("●").color(color));
            ui.label(RichText::new(&sample.spectrum.name).strong().color(color));
            ui.separator();
            ui.label(format!("T_S/T_B: {:.4}", comparison.scale_factor));
            ui.separator();
            ui.label(format!("Net total: {net_cps:.2} cps"));
            ui.separator();
            ui.label(format!("Neg bins: {}", comparison.negative_bin_count));
            ui.separator();
            ui.label(format!("Net min: {net_min_cps:.2} cps"));
        });
    }
}
