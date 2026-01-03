use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct AnalysisStore {
    // Map absolute path -> analysis data
    pub data: HashMap<PathBuf, Vec<f32>>,
}

impl AnalysisStore {
    /// Load from a binary file. Returns empty store if file doesn't exist.
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let bytes = fs::read(path).context("Failed to read analysis store file")?;
        let store = bincode::deserialize(&bytes).context("Failed to deserialize analysis store")?;
        Ok(store)
    }

    /// Save to a binary file.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).context("Failed to create analysis store directory")?;
        }
        let bytes = bincode::serialize(self).context("Failed to serialize analysis store")?;
        fs::write(path, bytes).context("Failed to write analysis store file")?;
        Ok(())
    }

    /// Insert or update a vector for a file path.
    pub fn insert(&mut self, path: PathBuf, analysis: Vec<f32>) {
        self.data.insert(path, analysis);
    }

    /// Retrieve vector for a file path.
    pub fn get(&self, path: &Path) -> Option<&Vec<f32>> {
        self.data.get(path)
    }

    /// Remove an entry (e.g. if file is deleted).
    pub fn remove(&mut self, path: &Path) {
        self.data.remove(path);
    }
}
