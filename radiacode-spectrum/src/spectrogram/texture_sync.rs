use egui::{Context, TextureOptions};

use crate::spectrogram::layout::{channels_in_energy_range, SpectrogramLayout};
use crate::spectrogram::model::SpectrogramDisplay;
use crate::spectrogram::state::SpectrogramState;
use crate::spectrogram::texture::source_columns;

pub fn sync_texture(ctx: &Context, state: &mut SpectrogramState, layout: SpectrogramLayout) {
    let energy_min = state.view_range.energy_min_kev;
    let energy_max = state.view_range.energy_max_kev;
    let total_rows = state.active_series().map(|series| series.row_count()).unwrap_or(0);
    let row_start = state
        .view_range
        .visible_start(total_rows, layout.display_rows);
    let channels_in_view = state
        .active_series()
        .map(|series| channels_in_energy_range(&series.energies_kev, energy_min, energy_max))
        .unwrap_or(layout.display_cols)
        .max(1);
    state
        .view_range
        .clamp_channels(channels_in_view, layout.display_cols);
    let source_cols = state
        .active_series()
        .map(|series| {
            source_columns(
                series,
                energy_min,
                energy_max,
                state.view_range.channel_start,
                layout.display_cols,
            )
        })
        .unwrap_or_default();
    let width = if source_cols.is_empty() {
        layout.display_cols.max(1)
    } else {
        source_cols.len()
    };
    let height = layout.display_rows.max(1);
    let needs_size_change =
        state.texture.image.width() != width || state.texture.image.height() != height;
    if needs_size_change {
        state.texture.dirty = true;
    }
    if state.texture.dirty {
        if state.active_series().is_some() {
            state.ensure_z_range();
        }
        let z_range = state.z_range;
        let series = match state.display {
            SpectrogramDisplay::Live => state.live_series.as_ref(),
            SpectrogramDisplay::Loaded => state.loaded_series.as_ref(),
        };
        if let (Some(series), Some(z_range)) = (series, z_range) {
            let visible = series.row_window(row_start, layout.display_rows);
            state.texture.rebuild(
                series,
                visible,
                &source_cols,
                layout.display_rows,
                &state.settings,
                &z_range,
            );
        } else {
            state.texture = crate::spectrogram::texture::SpectrogramTexture::new(width, height);
        }
    }
    if state.texture_handle.is_none() {
        state.texture_handle = Some(ctx.load_texture(
            "spectrogram_texture",
            state.texture.image.clone(),
            TextureOptions::NEAREST,
        ));
    }
    if state.texture.dirty {
        if let Some(texture) = state.texture_handle.as_mut() {
            texture.set(state.texture.image.clone(), TextureOptions::NEAREST);
        }
        state.texture.dirty = false;
    }
}
