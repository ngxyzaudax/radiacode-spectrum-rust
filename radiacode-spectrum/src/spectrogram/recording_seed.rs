use crate::energy::energy_grid;
use crate::model::SpectrumView;
use crate::spectrogram::capture::SpectrogramCapture;
use crate::spectrogram::model::{SpectrogramHeader, SpectrogramSeries};
use crate::spectrogram::storage::{header_now, RecordingWriter};

pub fn ensure_live_series(
    capture: &mut SpectrogramCapture,
    spectrum: &SpectrumView,
    device_serial: Option<&str>,
    energies_kev: &[f64],
) {
    if capture.live_series.is_some() {
        return;
    }
    let header = header_from_spectrum(
        spectrum,
        device_serial,
        energies_kev.len() as u32,
        capture.settings.capture_interval(),
    );
    capture.live_series = Some(SpectrogramSeries::new(header, energies_kev.to_vec()));
}

pub fn recording_header(
    capture: &SpectrogramCapture,
    spectrum: &SpectrumView,
    device_serial: Option<&str>,
    channel_count: u32,
) -> SpectrogramHeader {
    capture
        .live_series
        .as_ref()
        .map(|series| series.header.clone())
        .unwrap_or_else(|| {
            header_from_spectrum(
                spectrum,
                device_serial,
                channel_count,
                capture.settings.capture_interval(),
            )
        })
}

pub fn seed_writer_from_live(
    writer: &mut RecordingWriter,
    live_series: Option<&SpectrogramSeries>,
) -> std::io::Result<u32> {
    let Some(series) = live_series else {
        return Ok(0);
    };
    for row in &series.rows {
        writer.append_row(row)?;
    }
    Ok(series.rows.len() as u32)
}

fn header_from_spectrum(
    spectrum: &SpectrumView,
    device_serial: Option<&str>,
    channel_count: u32,
    interval_secs: f64,
) -> SpectrogramHeader {
    let grid = energy_grid(spectrum);
    header_now(
        spectrum.a0,
        spectrum.a1,
        spectrum.a2,
        channel_count,
        interval_secs,
        device_serial.map(str::to_string),
        grid.energies_kev,
    )
}
