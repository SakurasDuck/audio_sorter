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
    /// Predicted music genres (label, confidence), Top-3
    #[serde(default)]
    pub genres: Vec<(String, f32)>,
}

fn parse_metadata_from_filename(filename: &str) -> (Option<String>, Option<String>) {
    let stem = Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(filename);

    // Common patterns:
    // 1. "Artist - Title"
    // 2. "Title - Artist"
    // 3. "Title (feat. Artist)" - hard to distinguish from just Title with parens

    // Heuristic: Split by " - "
    let parts: Vec<&str> = stem.split(" - ").collect();
    if parts.len() == 2 {
        let p1 = parts[0].trim().to_string();
        let p2 = parts[1].trim().to_string();
        // Assume "Title - Artist" based on user example "Title-Artist"
        return (Some(p1), Some(p2));
    }

    // Fallback: Split by "-" (maybe no spaces) if " - " didn't match
    if parts.len() < 2 {
        let parts_dash: Vec<&str> = stem.split('-').collect();
        if parts_dash.len() >= 2 {
            // Handle "Title-Artist"
            // If multiple dashes, it's tricky.
            // E.g. "My-Song-Title-Artist"
            // Let's try to assume the last part is Artist?
            if let Some(artist) = parts_dash.last() {
                let title = parts_dash[..parts_dash.len() - 1].join("-");
                return (
                    Some(title.trim().to_string()),
                    Some(artist.trim().to_string()),
                );
            }
        }
    }

    (Some(stem.to_string()), None)
}

pub fn read_tags(path: &Path) -> Result<TrackMetadata> {
    let tagged_file_res = Probe::open(path)
        .context("Failed to open file for probing")
        .and_then(|p| p.read().context("Failed to read file tags"));

    let (mut title, mut artist, album) = match tagged_file_res {
        Ok(tagged_file) => {
            let tag = tagged_file
                .primary_tag()
                .or_else(|| tagged_file.first_tag());

            if let Some(t) = tag {
                (
                    t.title().map(|s| s.into_owned()).unwrap_or_default(),
                    t.artist().map(|s| s.into_owned()).unwrap_or_default(),
                    t.album().map(|s| s.into_owned()),
                )
            } else {
                (String::new(), String::new(), None)
            }
        }
        Err(_) => (String::new(), String::new(), None),
    };

    // Fallback to filename if title is empty
    if title.is_empty() || title == "Unknown Title" {
        if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
            let (f_title, f_artist) = parse_metadata_from_filename(filename);
            if let Some(t) = f_title {
                title = t;
            }
            if artist.is_empty() || artist == "Unknown Artist" {
                if let Some(a) = f_artist {
                    artist = a;
                }
            }
        }
    }

    // Ensure we don't return empty strings if we can parse something
    if title.is_empty() {
        title = "Unknown Title".to_string();
    }
    if artist.is_empty() {
        artist = "Unknown Artist".to_string();
    }

    Ok(TrackMetadata {
        title,
        artist,
        album,
        original_artist: None,
        original_title: None,
        duration: 0.0,
        fingerprint: None,
        genres: Vec::new(),
    })
}

/// Read tags from pre-loaded file data (avoids disk I/O)
/// Uses lofty 0.21+ Probe with Cursor for memory reading
pub fn read_tags_from_memory(data: &[u8], path: &Path) -> Result<TrackMetadata> {
    use std::io::Cursor;

    let cursor = Cursor::new(data);
    let tagged_file_res = Probe::new(cursor).read();

    let (mut title, mut artist, album) = match tagged_file_res {
        Ok(tagged_file) => {
            let tag = tagged_file
                .primary_tag()
                .or_else(|| tagged_file.first_tag());

            if let Some(t) = tag {
                (
                    t.title().map(|s| s.into_owned()).unwrap_or_default(),
                    t.artist().map(|s| s.into_owned()).unwrap_or_default(),
                    t.album().map(|s| s.into_owned()),
                )
            } else {
                (String::new(), String::new(), None)
            }
        }
        Err(_) => (String::new(), String::new(), None),
    };

    // Fallback to filename
    if title.is_empty() || title == "Unknown Title" {
        if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
            let (f_title, f_artist) = parse_metadata_from_filename(filename);
            if let Some(t) = f_title {
                title = t;
            }
            if artist.is_empty() || artist == "Unknown Artist" {
                if let Some(a) = f_artist {
                    artist = a;
                }
            }
        }
    }

    if title.is_empty() {
        title = "Unknown Title".to_string();
    }
    if artist.is_empty() {
        artist = "Unknown Artist".to_string();
    }

    Ok(TrackMetadata {
        title,
        artist,
        album,
        original_artist: None,
        original_title: None,
        duration: 0.0,
        fingerprint: None,
        genres: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_filename_simple_dash() {
        let (title, artist) = parse_metadata_from_filename("Song Title - Artist Name.mp3");
        assert_eq!(title.as_deref(), Some("Song Title"));
        assert_eq!(artist.as_deref(), Some("Artist Name"));
    }

    #[test]
    fn test_parse_filename_no_spaces_dash() {
        let (title, artist) = parse_metadata_from_filename("SongTitle-ArtistName.mp3");
        assert_eq!(title.as_deref(), Some("SongTitle"));
        assert_eq!(artist.as_deref(), Some("ArtistName"));
    }

    #[test]
    fn test_parse_filename_multiple_dashes() {
        // "Title-With-Dashes-Artist" -> Title="Title-With-Dashes", Artist="Artist"
        let (title, artist) = parse_metadata_from_filename("Title-With-Dashes-Artist.flac");
        assert_eq!(title.as_deref(), Some("Title-With-Dashes"));
        assert_eq!(artist.as_deref(), Some("Artist"));
    }

    #[test]
    fn test_parse_filename_no_dash() {
        let (title, artist) = parse_metadata_from_filename("JustTitle.wav");
        assert_eq!(title.as_deref(), Some("JustTitle"));
        assert_eq!(artist, None);
    }

    #[test]
    fn test_parse_filename_user_example() {
        // "BANG BANG BANG (뱅뱅뱅)-BIGBANG.flac"
        let (title, artist) = parse_metadata_from_filename("BANG BANG BANG (뱅뱅뱅)-BIGBANG.flac");
        assert_eq!(title.as_deref(), Some("BANG BANG BANG (뱅뱅뱅)"));
        assert_eq!(artist.as_deref(), Some("BIGBANG"));
    }
}
