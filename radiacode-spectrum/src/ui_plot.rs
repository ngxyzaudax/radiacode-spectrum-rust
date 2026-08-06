use egui::{RichText, Ui, Vec2b};
use egui_plot::{Bar, BarChart, Plot};

use crate::energy::{bar_energy_width, clamp_energy_range, energy_grid, ENERGY_MAX_KEV, ENERGY_MIN_KEV};
use crate::model::SpectrumView;
use crate::scale::{display_value, y_axis_top, YScale};
use crate::smooth::moving_average;
use crate::theme::{MUTED, SPECTRUM_BAR};

pub fn draw_spectrum_plot(
    ui: &mut Ui,
    spectrum: Option<&SpectrumView>,
    y_scale: YScale,
    smooth_window: usize,
) {
    let Some(spectrum) = spectrum else {
        ui.add_space(12.0);
        ui.label(
            RichText::new("No spectrum data yet. Connect a device to start capturing.")
                .color(MUTED),
        );
        return;
    };

    ui.horizontal(|ui| {
        ui.label(format!(
            "Live time: {:.1}s",
            spectrum.duration.as_secs_f64()
        ));
        ui.separator();
        ui.label(format!("Total counts: {}", spectrum.total_counts));
        ui.separator();
        ui.label(format!("Channels: {}", spectrum.counts.len()));
        ui.separator();
        ui.label(format!(
            "E= {:.2}+{:.3}·ch+{:.5}·ch² keV",
            spectrum.a0, spectrum.a1, spectrum.a2
        ));
    });

    ui.add_space(8.0);
    let bars = build_spectrum_bars(spectrum, y_scale, smooth_window);
    let peak = bars.iter().map(|bar| bar.value).fold(0.0_f64, f64::max);
    let y_top = y_axis_top(peak, y_scale);

    Plot::new("spectrum_plot_kev")
        .allow_zoom(true)
        .allow_drag(true)
        .allow_scroll(true)
        .auto_bounds(Vec2b::new(false, false))
        .default_x_bounds(ENERGY_MIN_KEV, ENERGY_MAX_KEV)
        .include_y(0.0)
        .x_axis_label("Energy (keV)")
        .y_axis_label(y_axis_label(y_scale))
        .show(ui, |plot_ui| {
            let bounds = plot_ui.plot_bounds();
            let (min_x, max_x) = clamp_energy_range(bounds.min()[0], bounds.max()[0]);
            plot_ui.set_plot_bounds_x(min_x..=max_x);
            plot_ui.set_plot_bounds_y(0.0..=y_top);
            if !bars.is_empty() {
                plot_ui.bar_chart(
                    BarChart::new("spectrum", bars)
                        .vertical()
                        .color(SPECTRUM_BAR)
                        .element_formatter(Box::new(move |bar, _| bar_hover_label(bar, y_scale))),
                );
            }
        });
}

fn y_axis_label(scale: YScale) -> &'static str {
    match scale {
        YScale::Linear => "Counts",
        YScale::Logarithmic => "Counts (log10)",
    }
}

fn bar_hover_label(bar: &Bar, y_scale: YScale) -> String {
    let counts = hover_counts(bar.value, y_scale);
    format!("{:.1} keV\n{counts} counts", bar.argument)
}

fn hover_counts(displayed: f64, y_scale: YScale) -> String {
    match y_scale {
        YScale::Linear => format!("{displayed:.1}"),
        YScale::Logarithmic => format!("{:.1}", 10_f64.powf(displayed)),
    }
}

fn build_spectrum_bars(
    spectrum: &SpectrumView,
    y_scale: YScale,
    smooth_window: usize,
) -> Vec<Bar> {
    let smoothed = moving_average(&spectrum.counts, smooth_window);
    let grid = energy_grid(spectrum);
    grid.energies_kev
        .iter()
        .enumerate()
        .map(|(index, &energy)| {
            let height = display_value(smoothed[grid.indices[index]], y_scale);
            Bar::new(energy, height)
                .width(bar_energy_width(&grid.energies_kev, index, spectrum.a1 as f64))
                .fill(SPECTRUM_BAR)
        })
        .collect()
}
