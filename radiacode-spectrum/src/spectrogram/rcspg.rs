use std::path::Path;

use crate::spectrogram::model::{RowKind, SpectrogramHeader, SpectrogramRow, SpectrogramSeries};

pub fn export_recording(
    path: &Path,
    series: &SpectrogramSeries,
    name: &str,
    comment: &str,
) -> std::io::Result<()> {
    let header_line = build_header_line(series, name, comment);
    let spectrum_line = encode_historical_spectrum(&series);
    let mut body = String::new();
    for row in &series.rows {
        body.push_str(&format_row(row));
        body.push('\n');
    }
    let content = format!("{header_line}\n{spectrum_line}\n{body}");
    fs::write(path, content.as_bytes())?;
    Ok(())
}

pub fn import_recording(path: &Path) -> std::io::Result<SpectrogramSeries> {
    let content = fs::read_to_string(path)?;
    let mut lines = content.lines();
    let header_line = lines
        .next()
        .ok_or_else(|| invalid("missing header"))?;
    if !header_line.starts_with("Spectrogram:") {
        return Err(invalid("not an rcspg file"));
    }
    let spectrum_line = lines
        .next()
        .ok_or_else(|| invalid("missing spectrum line"))?;
    let hist = decode_historical_spectrum(spectrum_line)?;
    let channels = parse_header_field(header_line, "Channels")
        .and_then(|value| value.parse().ok())
        .unwrap_or(hist.counts.len() as u32);
    let interval = parse_header_field(header_line, "Accumulation time")
        .and_then(|value| value.parse().ok())
        .unwrap_or(60.0);
    let serial = parse_header_field(header_line, "Device serial").map(str::to_string);
    let created = parse_header_field(header_line, "Time")
        .unwrap_or("unknown")
        .to_string();
    let a0 = hist.calibration[0];
    let a1 = hist.calibration[1];
    let a2 = hist.calibration[2];
    let energies_kev: Vec<f64> = (0..channels as usize)
        .map(|index| {
            let ch = index as f64;
            f64::from(a0) + f64::from(a1) * ch + f64::from(a2) * ch * ch
        })
        .collect();
    let header = SpectrogramHeader {
        created_at: created,
        a0,
        a1,
        a2,
        channel_count: channels,
        interval_secs: interval,
        device_serial: serial,
        energies_kev: energies_kev.clone(),
    };
    let mut rows = Vec::new();
    let mut elapsed = 0.0;
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let values = parse_number_list(line);
        if values.len() < 2 {
            continue;
        }
        let row_interval = if values.len() >= 4 {
            values[1] as f64
        } else {
            values[1] as f64
        };
        let kind = if values.len() >= 4 {
            let raw_total = values
                .iter()
                .skip(4)
                .take(channels as usize)
                .map(|&value| value as u64)
                .sum();
            RowKind::from_storage_tag(values[2] as u8, values[3] as f64, raw_total)
        } else {
            RowKind::Normal
        };
        let count_offset = if values.len() >= 4 { 4 } else { 2 };
        let counts: Vec<u32> = values
            .iter()
            .skip(count_offset)
            .take(channels as usize)
            .map(|&value| value as u32)
            .collect();
        rows.push(SpectrogramRow {
            elapsed_secs: elapsed,
            interval_secs: row_interval,
            kind,
            counts,
        });
        elapsed += row_interval;
    }
    Ok(SpectrogramSeries {
        header,
        energies_kev,
        rows,
    })
}

struct HistoricalSpectrum {
    duration: u32,
    calibration: [f32; 3],
    counts: Vec<u32>,
}

fn build_header_line(series: &SpectrogramSeries, name: &str, comment: &str) -> String {
    format!(
        "Spectrogram: {name}\tTime: {}\tTimestamp: 0\tAccumulation time: {}\tChannels: {}\tDevice serial: {}\tFlags: 0\tComment: {comment}",
        series.header.created_at,
        series.header.interval_secs as u32,
        series.header.channel_count,
        series.header.device_serial.as_deref().unwrap_or(""),
    )
}

fn encode_historical_spectrum(series: &SpectrogramSeries) -> String {
    let cumulative: Vec<u32> = if let Some(first) = series.rows.first() {
        first.counts.clone()
    } else {
        vec![0; series.header.channel_count as usize]
    };
    let duration = series.duration_secs().max(0.0) as u32;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&duration.to_le_bytes());
    for value in [series.header.a0, series.header.a1, series.header.a2] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in &cumulative {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    let hex: String = bytes
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ");
    format!("Spectrum: {hex}")
}

fn decode_historical_spectrum(line: &str) -> std::io::Result<HistoricalSpectrum> {
    let hex = line
        .trim()
        .strip_prefix("Spectrum:")
        .ok_or_else(|| invalid("missing spectrum prefix"))?
        .replace(' ', "");
    let bytes: Vec<u8> = (0..hex.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&hex[index..index + 2], 16))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| invalid(&error.to_string()))?;
    if bytes.len() < 16 {
        return Err(invalid("spectrum line too short"));
    }
    let duration = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
    let calibration = [
        f32::from_le_bytes(bytes[4..8].try_into().unwrap()),
        f32::from_le_bytes(bytes[8..12].try_into().unwrap()),
        f32::from_le_bytes(bytes[12..16].try_into().unwrap()),
    ];
    let counts = bytes[16..]
        .chunks_exact(4)
        .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect();
    Ok(HistoricalSpectrum {
        duration,
        calibration,
        counts,
    })
}

fn format_row(row: &SpectrogramRow) -> String {
    let mut parts = vec![
        (row.elapsed_secs as u64).to_string(),
        row.interval_secs.round().to_string(),
        row.kind.storage_tag().to_string(),
        row.kind.storage_extra().to_string(),
    ];
    parts.extend(row.counts.iter().map(|value| value.to_string()));
    parts.join(" ")
}

fn parse_header_field<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    line.split('\t').find_map(|field| {
        let mut parts = field.splitn(2, ':');
        let name = parts.next()?.trim();
        if name == key {
            parts.next().map(str::trim)
        } else {
            None
        }
    })
}

fn parse_number_list(line: &str) -> Vec<i64> {
    line.split_whitespace()
        .filter_map(|token| token.parse().ok())
        .collect()
}

fn invalid(message: &str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, message.to_string())
}

use std::fs;

#[cfg(test)]
mod tests {
    use super::{export_recording, import_recording};
    use crate::spectrogram::model::{RowKind, SpectrogramHeader, SpectrogramSeries};
    use tempfile::tempdir;

    #[test]
    fn rcspg_round_trip() {
        let header = SpectrogramHeader {
            created_at: "2024-01-01 00:00:00".into(),
            a0: 0.0,
            a1: 1.0,
            a2: 0.0,
            channel_count: 3,
            interval_secs: 60.0,
            device_serial: Some("RC-TEST".into()),
            energies_kev: vec![0.0, 1.0, 2.0],
        };
        let mut series = SpectrogramSeries::new(header, vec![0.0, 1.0, 2.0]);
        series.push_row(vec![1, 2, 3], 60.0, RowKind::Normal, 1000);
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.rcspg");
        export_recording(&path, &series, "test", "note").unwrap();
        let loaded = import_recording(&path).unwrap();
        assert_eq!(loaded.rows.len(), 1);
        assert_eq!(loaded.rows[0].counts, vec![1, 2, 3]);
    }
}
