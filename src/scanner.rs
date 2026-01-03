use anyhow::Result;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn scan_directory(path: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let valid_extensions: HashSet<&str> =
        ["mp3", "flac", "wav", "m4a", "ogg"].into_iter().collect();

    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                if valid_extensions.contains(ext.to_lowercase().as_str()) {
                    files.push(path.to_path_buf());
                }
            }
        }
    }
    Ok(files)
}
