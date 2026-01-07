use anyhow::{Context, Result};
use lofty::file::TaggedFileExt;
use lofty::probe::Probe;
use lofty::tag::Accessor;
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
    let tagged_file = Probe::open(path)
        .context("Failed to open file for probing")?
        .read()
        .context("Failed to read file tags")?;

    let tag = tagged_file
        .primary_tag()
        .or_else(|| tagged_file.first_tag());

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
        original_artist: None,
        original_title: None,
        duration: 0.0,
        fingerprint: None,
    })
}

/// Read tags from pre-loaded file data (avoids disk I/O)
/// Uses lofty 0.21+ Probe with Cursor for memory reading
pub fn read_tags_from_memory(data: &[u8], _path: &Path) -> Result<TrackMetadata> {
    use std::io::Cursor;

    let cursor = Cursor::new(data);
    let tagged_file = Probe::new(cursor)
        .read()
        .context("Failed to read tags from memory")?;

    let tag = tagged_file
        .primary_tag()
        .or_else(|| tagged_file.first_tag());

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
        original_artist: None,
        original_title: None,
        duration: 0.0,
        fingerprint: None,
    })
}
