use anyhow::{Context, Result};
use reqwest::blocking::Client;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct MBRecordingResponse {
    pub id: String,
    pub title: String,
    #[serde(rename = "artist-credit")]
    pub artist_credit: Option<Vec<ArtistCredit>>,
    pub relations: Option<Vec<Relation>>,
}

#[derive(Debug, Deserialize)]
pub struct ArtistCredit {
    pub name: String,
    pub artist: Option<MBArtist>,
}

#[derive(Debug, Deserialize)]
pub struct MBArtist {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct Relation {
    #[serde(rename = "type")]
    pub rel_type: String, // e.g., "performance"
    pub work: Option<MBWork>,
}

#[derive(Debug, Deserialize)]
pub struct MBWork {
    pub id: String,
    pub title: String,
    pub relations: Option<Vec<Relation>>, // To find other recordings of this work
}

// Struct for Work lookup response which contains recordings
#[derive(Debug, Deserialize)]
pub struct MBWorkResponse {
    pub id: String,
    pub title: String,
    pub relations: Option<Vec<WorkRelation>>,
}

#[derive(Debug, Deserialize)]
pub struct WorkRelation {
    #[serde(rename = "type")]
    pub rel_type: String,
    pub recording: Option<MBRecordingMinimal>,
    pub begin: Option<String>, // Date, e.g. "1988-01-01"
}

#[derive(Debug, Deserialize)]
pub struct MBRecordingMinimal {
    pub id: String,
    pub title: String,
    #[serde(rename = "artist-credit")]
    pub artist_credit: Option<Vec<ArtistCredit>>,
}

pub fn fetch_recording_details(client: &Client, recording_id: &str) -> Result<MBRecordingResponse> {
    let url = format!(
        "https://musicbrainz.org/ws/2/recording/{}?inc=work-rels+artist-credits&fmt=json",
        recording_id
    );

    let resp = client
        .get(&url)
        .header("User-Agent", "AudioSorter/0.1.0 ( myemail@example.com )") // Replace with real info or arg
        .send()
        .context("Failed to query MusicBrainz")?;

    // Sleep to respect rate limits (1 req/sec)
    std::thread::sleep(std::time::Duration::from_secs(1));

    if !resp.status().is_success() {
        return Err(anyhow::anyhow!("MusicBrainz API error: {}", resp.status()));
    }

    let data: MBRecordingResponse = resp.json()?;
    Ok(data)
}

pub fn fetch_work_recordings(client: &Client, work_id: &str) -> Result<MBWorkResponse> {
    // Get work and linked recordings
    let url = format!(
        "https://musicbrainz.org/ws/2/work/{}?inc=recording-rels+artist-credits&fmt=json",
        work_id
    );

    let resp = client
        .get(&url)
        .header("User-Agent", "AudioSorter/0.1.0 ( myemail@example.com )")
        .send()
        .context("Failed to query MusicBrainz Work")?;

    std::thread::sleep(std::time::Duration::from_secs(1));

    let data: MBWorkResponse = resp.json()?;
    Ok(data)
}
