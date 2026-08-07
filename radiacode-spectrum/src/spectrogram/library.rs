use std::fs;
use std::path::{Path, PathBuf};

use crate::spectrogram::library_meta::{load_meta, save_meta, LibraryMeta};
use crate::spectrogram::model::SpectrogramSeries;
use crate::spectrogram::rcspg;
use crate::spectrogram::storage::{
    ensure_dir, load_recording, spectrograms_dir, timestamp_filename, write_recording,
    RecordingWriter,
};

pub fn rename_entry(path: &Path, name: &str) -> Result<(), String> {
    let mut meta = load_meta(path, name);
    meta.name = name.to_string();
    save_meta(path, &meta).map_err(|error| error.to_string())
}

pub fn set_comment(path: &Path, comment: &str) -> Result<(), String> {
    let fallback = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("recording");
    let mut meta = load_meta(path, fallback);
    meta.comment = comment.to_string();
    save_meta(path, &meta).map_err(|error| error.to_string())
}

pub fn delete_entry(path: &Path) -> Result<(), String> {
    fs::remove_file(path).map_err(|error| error.to_string())?;
    let meta_path = crate::spectrogram::library_meta::meta_path(path);
    if meta_path.exists() {
        let _ = fs::remove_file(meta_path);
    }
    Ok(())
}

pub fn export_rcspg(path: &Path, destination: &Path) -> Result<(), String> {
    let series = load_recording(path).map_err(|error| error.to_string())?;
    let meta = load_meta(
        path,
        path.file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("recording"),
    );
    rcspg::export_recording(destination, &series, &meta.name, &meta.comment)
        .map_err(|error| error.to_string())
}

pub fn import_rcspg(source: &Path, recordings_dir: &str) -> Result<PathBuf, String> {
    let series = rcspg::import_recording(source).map_err(|error| error.to_string())?;
    let dir = ensure_dir(recordings_dir).map_err(|error| error.to_string())?;
    let path = dir.join(timestamp_filename());
    write_recording(&path, &series).map_err(|error| error.to_string())?;
    let meta = LibraryMeta {
        name: source
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("import")
            .to_string(),
        comment: String::new(),
    };
    save_meta(&path, &meta).map_err(|error| error.to_string())?;
    Ok(path)
}

pub fn auto_save_snapshot(
    series: &SpectrogramSeries,
    writer: Option<&RecordingWriter>,
    recordings_dir: &str,
) -> std::io::Result<PathBuf> {
    let dir = spectrograms_dir(recordings_dir).join("autosave");
    fs::create_dir_all(&dir)?;
    let path = dir.join(format!("autosave_{}", timestamp_filename()));
    write_recording(&path, series)?;
    if let Some(writer) = writer {
        let fallback = writer
            .path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("recording");
        let _ = save_meta(
            &path,
            &LibraryMeta {
                name: format!("{fallback} (autosave)"),
                comment: String::new(),
            },
        );
    }
    Ok(path)
}
