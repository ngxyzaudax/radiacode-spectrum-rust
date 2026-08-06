use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LibraryMeta {
    pub name: String,
    pub comment: String,
}

pub fn meta_path(recording_path: &Path) -> PathBuf {
    recording_path.with_extension("rcwf.meta.json")
}

pub fn load_meta(recording_path: &Path, fallback_name: &str) -> LibraryMeta {
    let path = meta_path(recording_path);
    let Ok(bytes) = fs::read(&path) else {
        return LibraryMeta {
            name: fallback_name.to_string(),
            comment: String::new(),
        };
    };
    serde_json::from_slice(&bytes).unwrap_or(LibraryMeta {
        name: fallback_name.to_string(),
        comment: String::new(),
    })
}

pub fn save_meta(recording_path: &Path, meta: &LibraryMeta) -> std::io::Result<()> {
    let path = meta_path(recording_path);
    let bytes = serde_json::to_vec_pretty(meta)?;
    fs::write(path, bytes)
}
