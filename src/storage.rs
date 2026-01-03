use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::organizer::TrackMetadata;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct AudioLibrary {
    pub files: HashMap<PathBuf, IndexedTrack>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IndexedTrack {
    pub path: PathBuf,
    pub file_size: u64,
    pub modified_time: u64, // UNIX timestamp (seconds)
    pub scanned_at: u64,    // UNIX timestamp (seconds)
    pub metadata: TrackMetadata,
}

impl AudioLibrary {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(path).context("Failed to read library index file")?;
        let library =
            serde_json::from_str(&content).context("Failed to parse library index JSON")?;
        Ok(library)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let content =
            serde_json::to_string_pretty(self).context("Failed to serialize library index")?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).context("Failed to create library index directory")?;
        }
        fs::write(path, content).context("Failed to write library index file")?;
        Ok(())
    }

    pub fn find_duplicates(&self) -> Vec<Vec<IndexedTrack>> {
        let mut groups: HashMap<String, Vec<IndexedTrack>> = HashMap::new();

        for track in self.files.values() {
            if let Some(fp) = &track.metadata.fingerprint {
                groups.entry(fp.clone()).or_default().push(track.clone());
            }
        }

        groups.into_values().filter(|g| g.len() > 1).collect()
    }
}
