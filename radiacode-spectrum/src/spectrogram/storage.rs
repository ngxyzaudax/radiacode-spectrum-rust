use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use serde_json;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::spectrogram::library_meta::load_meta;
use crate::spectrogram::model::{
    RecordingEntry, RowKind, SpectrogramHeader, SpectrogramRow, SpectrogramSeries,
};

const MAGIC: &[u8; 4] = b"RCWF";
const VERSION_V1: u32 = 1;
const VERSION_V2: u32 = 2;
const VERSION_CURRENT: u32 = VERSION_V2;

pub fn default_spectrograms_dir() -> PathBuf {
    ProjectDirs::from("com", "radiacode", "radiacode-spectrum")
        .map(|dirs| dirs.data_dir().join("spectrograms"))
        .unwrap_or_else(|| PathBuf::from("spectrograms"))
}

pub fn spectrograms_dir(configured: &str) -> PathBuf {
    let trimmed = configured.trim();
    if trimmed.is_empty() {
        default_spectrograms_dir()
    } else {
        PathBuf::from(trimmed)
    }
}

fn legacy_spectrograms_dir() -> PathBuf {
    ProjectDirs::from("com", "radiacode", "radiacode-spectrum")
        .map(|dirs| dirs.data_dir().join("waterfalls"))
        .unwrap_or_else(|| PathBuf::from("waterfalls"))
}

pub fn ensure_dir(configured: &str) -> std::io::Result<PathBuf> {
    let dir = spectrograms_dir(configured);
    fs::create_dir_all(&dir)?;
    if configured.trim().is_empty() {
        let legacy = legacy_spectrograms_dir();
        if legacy.exists() && legacy != dir {
            migrate_legacy_recordings(&legacy, &dir)?;
        }
    }
    Ok(dir)
}

fn migrate_legacy_recordings(from: &Path, to: &Path) -> std::io::Result<()> {
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("rcwf") {
            continue;
        }
        let Some(name) = path.file_name() else {
            continue;
        };
        let destination = to.join(name);
        if !destination.exists() {
            let _ = fs::copy(&path, &destination);
        }
    }
    Ok(())
}

use time::macros::format_description;

pub fn timestamp_filename() -> String {
    let now = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());
    format!(
        "{}.rcwf",
        now.format(format_description!("[year]-[month]-[day]_[hour]-[minute]-[second]"))
            .unwrap_or_else(|_| "recording".into())
    )
}

pub fn list_recordings(configured: &str) -> std::io::Result<Vec<RecordingEntry>> {
    let dir = ensure_dir(configured)?;
    let mut entries = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("rcwf") {
            continue;
        }
        if let Some(item) = build_entry(path) {
            entries.push(item);
        }
    }
    entries.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    Ok(entries)
}

pub(crate) fn build_entry(path: PathBuf) -> Option<RecordingEntry> {
    let fallback = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("recording")
        .to_string();
    let series = load_recording(&path).ok()?;
    let meta = load_meta(&path, &fallback);
    Some(RecordingEntry {
        path,
        name: meta.name,
        comment: meta.comment,
        created_at: series.header.created_at.clone(),
        device_serial: series.header.device_serial.clone(),
        interval_secs: series.header.interval_secs,
        row_count: series.rows.len() as u32,
        channel_count: series.header.channel_count,
    })
}

pub struct RecordingWriter {
    file: File,
    row_count_offset: u64,
    pub path: PathBuf,
    pub row_count: u32,
    version: u32,
    channel_count: u32,
}

impl RecordingWriter {
    pub fn create(path: PathBuf, header: &SpectrogramHeader) -> std::io::Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = File::create(&path)?;
        let header_json = serde_json::to_vec(header)?;
        file.write_all(MAGIC)?;
        write_u32(&mut file, VERSION_CURRENT)?;
        write_u32(&mut file, header_json.len() as u32)?;
        file.write_all(&header_json)?;
        write_u32(&mut file, header.channel_count)?;
        let row_count_offset = file.stream_position()?;
        write_u32(&mut file, 0)?;
        Ok(Self {
            file,
            row_count_offset,
            path,
            row_count: 0,
            version: VERSION_CURRENT,
            channel_count: header.channel_count,
        })
    }

    pub fn append_row(&mut self, row: &SpectrogramRow) -> std::io::Result<()> {
        for value in &row.counts {
            write_u32(&mut self.file, *value)?;
        }
        if self.version >= VERSION_V2 {
            write_f64(&mut self.file, row.interval_secs)?;
            self.file.write_all(&[row.kind.storage_tag()])?;
            write_f64(&mut self.file, row.kind.storage_extra())?;
        }
        self.row_count += 1;
        Ok(())
    }

    pub fn finalize(mut self) -> std::io::Result<PathBuf> {
        self.file.seek(SeekFrom::Start(self.row_count_offset))?;
        write_u32(&mut self.file, self.row_count)?;
        self.file.sync_all()?;
        Ok(self.path)
    }
}

pub fn open_recording_append(path: PathBuf) -> std::io::Result<RecordingWriter> {
    let file = OpenOptions::new().append(true).open(&path)?;
    let series = load_recording(&path)?;
    let row_count = series.rows.len() as u32;
    let mut inner = File::open(&path)?;
    let mut magic = [0_u8; 4];
    inner.read_exact(&mut magic)?;
    let version = read_u32(&mut inner)?;
    let header_len = read_u32(&mut inner)? as usize;
    inner.seek(SeekFrom::Current(header_len as i64))?;
    let channel_count = read_u32(&mut inner)?;
    let row_count_offset = inner.stream_position()?;
    Ok(RecordingWriter {
        file,
        row_count_offset,
        path,
        row_count,
        version,
        channel_count,
    })
}

pub fn write_recording(path: &Path, series: &SpectrogramSeries) -> std::io::Result<()> {
    let mut writer = RecordingWriter::create(path.to_path_buf(), &series.header)?;
    for row in &series.rows {
        writer.append_row(row)?;
    }
    writer.finalize()?;
    Ok(())
}

pub fn load_recording(path: &Path) -> std::io::Result<SpectrogramSeries> {
    let mut file = File::open(path)?;
    let mut magic = [0_u8; 4];
    file.read_exact(&mut magic)?;
    if &magic != MAGIC {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "invalid spectrogram file magic",
        ));
    }
    let version = read_u32(&mut file)?;
    if version != VERSION_V1 && version != VERSION_V2 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "unsupported spectrogram file version",
        ));
    }
    let header_len = read_u32(&mut file)? as usize;
    let mut header_bytes = vec![0_u8; header_len];
    file.read_exact(&mut header_bytes)?;
    let header: SpectrogramHeader = serde_json::from_slice(&header_bytes)?;
    let channel_count = read_u32(&mut file)? as usize;
    let row_count = read_u32(&mut file)? as usize;
    let mut rows = Vec::with_capacity(row_count);
    if version == VERSION_V1 {
        let mut payload = vec![0_u8; row_count * channel_count * 4];
        file.read_exact(&mut payload)?;
        for row_index in 0..row_count {
            let start = row_index * channel_count * 4;
            let counts = payload[start..start + channel_count * 4]
                .chunks_exact(4)
                .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                .collect();
            rows.push(SpectrogramRow {
                elapsed_secs: 0.0,
                interval_secs: header.interval_secs,
                kind: RowKind::Normal,
                counts,
            });
        }
    } else {
        for _ in 0..row_count {
            let counts = read_row_counts(&mut file, channel_count)?;
            let interval_secs = read_f64(&mut file)?;
            let mut kind_tag = [0_u8; 1];
            file.read_exact(&mut kind_tag)?;
            let extra = read_f64(&mut file)?;
            let raw_total = counts.iter().map(|&value| value as u64).sum();
            rows.push(SpectrogramRow {
                elapsed_secs: 0.0,
                interval_secs,
                kind: RowKind::from_storage_tag(kind_tag[0], extra, raw_total),
                counts,
            });
        }
    }
    assign_elapsed_secs(&mut rows);
    let energies_kev = header.energies_kev.clone();
    Ok(SpectrogramSeries {
        header,
        energies_kev,
        rows,
    })
}

fn assign_elapsed_secs(rows: &mut [SpectrogramRow]) {
    let mut elapsed = 0.0;
    for row in rows.iter_mut() {
        row.elapsed_secs = elapsed;
        elapsed += row.interval_secs;
    }
}

fn read_row_counts(reader: &mut File, channel_count: usize) -> std::io::Result<Vec<u32>> {
    let mut counts = Vec::with_capacity(channel_count);
    for _ in 0..channel_count {
        counts.push(read_u32(reader)?);
    }
    Ok(counts)
}

pub fn header_now(
    a0: f32,
    a1: f32,
    a2: f32,
    channel_count: u32,
    interval_secs: f64,
    device_serial: Option<String>,
    energies_kev: Vec<f64>,
) -> SpectrogramHeader {
    let created_at = match OffsetDateTime::now_local() {
        Ok(now) => now,
        Err(_) => OffsetDateTime::now_utc(),
    }
    .format(&Rfc3339)
    .unwrap_or_else(|_| "unknown".into());
    SpectrogramHeader {
        created_at,
        a0,
        a1,
        a2,
        channel_count,
        interval_secs,
        device_serial,
        energies_kev,
    }
}

fn write_u32(writer: &mut File, value: u32) -> std::io::Result<()> {
    writer.write_all(&value.to_le_bytes())
}

fn read_u32(reader: &mut File) -> std::io::Result<u32> {
    let mut bytes = [0_u8; 4];
    reader.read_exact(&mut bytes)?;
    Ok(u32::from_le_bytes(bytes))
}

fn write_f64(writer: &mut File, value: f64) -> std::io::Result<()> {
    writer.write_all(&value.to_le_bytes())
}

fn read_f64(reader: &mut File) -> std::io::Result<f64> {
    let mut bytes = [0_u8; 8];
    reader.read_exact(&mut bytes)?;
    Ok(f64::from_le_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::{header_now, load_recording, RecordingWriter, VERSION_V1};
    use crate::spectrogram::model::{RowKind, SpectrogramRow};
    use tempfile::tempdir;

    fn gap_row(counts: Vec<u32>) -> SpectrogramRow {
        let raw_total = counts.iter().map(|&value| value as u64).sum();
        SpectrogramRow {
            elapsed_secs: 0.0,
            interval_secs: 45.0,
            kind: RowKind::GapRecovery {
                offline_secs: 45.0,
                raw_total,
            },
            counts,
        }
    }

    #[test]
    fn round_trip_recording_v2() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.rcwf");
        let header = header_now(0.0, 1.0, 0.0, 2, 5.0, None, vec![100.0, 200.0]);
        let mut writer = RecordingWriter::create(path.clone(), &header).unwrap();
        writer
            .append_row(&SpectrogramRow {
                elapsed_secs: 0.0,
                interval_secs: 5.0,
                kind: RowKind::Normal,
                counts: vec![1, 2],
            })
            .unwrap();
        writer.append_row(&gap_row(vec![100, 200])).unwrap();
        writer.finalize().unwrap();
        let loaded = load_recording(&path).unwrap();
        assert_eq!(loaded.rows.len(), 2);
        assert_eq!(loaded.rows[0].counts, vec![1, 2]);
        assert!(matches!(loaded.rows[1].kind, RowKind::GapRecovery { .. }));
        assert!((loaded.rows[1].interval_secs - 45.0).abs() < 0.001);
        assert!((loaded.duration_secs() - 50.0).abs() < 0.001);
    }

    #[test]
    fn v1_loader_treats_rows_as_normal() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.rcwf");
        let header = header_now(0.0, 1.0, 0.0, 2, 5.0, None, vec![100.0, 200.0]);
        let mut file = std::fs::File::create(&path).unwrap();
        use std::io::Write;
        file.write_all(b"RCWF").unwrap();
        file.write_all(&VERSION_V1.to_le_bytes()).unwrap();
        let header_json = serde_json::to_vec(&header).unwrap();
        file.write_all(&(header_json.len() as u32).to_le_bytes())
            .unwrap();
        file.write_all(&header_json).unwrap();
        file.write_all(&2_u32.to_le_bytes()).unwrap();
        file.write_all(&1_u32.to_le_bytes()).unwrap();
        file.write_all(&1_u32.to_le_bytes()).unwrap();
        file.write_all(&2_u32.to_le_bytes()).unwrap();
        drop(file);
        let loaded = load_recording(&path).unwrap();
        assert_eq!(loaded.rows.len(), 1);
        assert!(matches!(loaded.rows[0].kind, RowKind::Normal));
        assert!((loaded.rows[0].interval_secs - 5.0).abs() < 0.001);
    }
}
