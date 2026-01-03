use anyhow::{Context, Result};
use reqwest::blocking::Client;
use serde::Deserialize; // Using blocking for simplicity in this flow, or async if main is async

#[derive(Debug, Deserialize)]
pub struct AcoustIdResponse {
    pub status: String,
    pub results: Option<Vec<AcoustIdResult>>,
}

#[derive(Debug, Deserialize)]
pub struct AcoustIdResult {
    pub id: String,
    pub score: f64,
    pub recordings: Option<Vec<Recording>>,
}

#[derive(Debug, Deserialize)]
pub struct Recording {
    pub id: String,
    pub title: Option<String>,
    pub artists: Option<Vec<Artist>>,
}

#[derive(Debug, Deserialize)]
pub struct Artist {
    pub id: String,
    pub name: String,
}

pub fn lookup_fingerprint(
    client_id: &str,
    duration: f64,
    fingerprint: &str,
) -> Result<AcoustIdResponse> {
    let client = Client::new();
    let url = "https://api.acoustid.org/v2/lookup";

    let params = [
        ("client", client_id),
        ("meta", "recordings+compress"), // requesting recordings
        ("duration", &duration.round().to_string()),
        ("fingerprint", fingerprint),
    ];

    let resp = client
        .post(url)
        .form(&params)
        .send()
        .context("Failed to send request to AcoustID")?;

    if !resp.status().is_success() {
        return Err(anyhow::anyhow!(
            "AcoustID API returned error: {}",
            resp.status()
        ));
    }

    let parsed: AcoustIdResponse = resp.json().context("Failed to parse AcoustID response")?;
    Ok(parsed)
}
