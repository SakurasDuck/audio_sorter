use anyhow::{Context, Result};
use lofty::{Accessor, TaggedFileExt};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct TrackMetadata {
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub original_artist: Option<String>, // For covers
    pub original_title: Option<String>,  // For covers
    pub duration: f64,                   // Duration in seconds
    pub fingerprint: Option<String>,     // Chromaprint fingerprint
}

pub fn read_tags(path: &Path) -> Result<TrackMetadata> {
    let probed = lofty::Probe::open(path)
        .context("Failed to open file for probing")?
        .read()
        .context("Failed to read file tags")?;

    let tag = probed.primary_tag().or_else(|| probed.first_tag());

    let (title, artist, album) = if let Some(t) = tag {
        (
            t.title().map(|s| s.into_owned()).unwrap_or_default(),
            t.artist().map(|s| s.into_owned()).unwrap_or_default(),
            t.album().map(|s| s.into_owned()),
        )
    } else {
        (String::new(), String::new(), None)
    };

    Ok(TrackMetadata {
        title,
        artist,
        album,
        original_artist: None, // Cannot know from local tags alone usually
        original_title: None,
        duration: 0.0, // Will be filled by scanner/fingerprinter
        fingerprint: None,
    })
}
