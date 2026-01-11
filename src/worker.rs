use anyhow::{Context, Result};
use std::path::Path;

use crate::audio_decoder;
use crate::fingerprint;
use crate::genre_classifier;
use crate::organizer::{self, TrackMetadata};
use crate::ScanArgs;

// bliss_audio::Song::analyze is used for in-memory melody analysis

pub fn process_file(path: &Path, _args: &ScanArgs) -> Result<(TrackMetadata, Option<Vec<f32>>)> {
    // Decode audio once using our unified decoder
    let decoded = audio_decoder::decode_audio(path).context("Failed to decode audio file")?;

    // Compute fingerprint from decoded samples (no re-reading file)
    let fp = fingerprint::compute_fingerprint_from_decoded(&decoded)
        .context("Fingerprint generation failed")?;

    let duration = decoded.duration_secs;

    // Build metadata - Always use local tags
    let mut meta = organizer::read_tags(path).context("Failed to read local tags")?;
    meta.duration = duration;
    meta.fingerprint = Some(fp.clone());

    // Melody Analysis (Bliss) - from memory using pre-decoded samples
    let bliss_samples = decoded.to_bliss_samples();
    let analysis = match bliss_audio::Song::analyze(&bliss_samples) {
        Ok(bliss_analysis) => Some(bliss_analysis.as_vec()),
        Err(_e) => None,
    };

    // Genre Classification (if models are loaded)
    // Use bliss_samples (mono f32 @ 22050Hz)
    if genre_classifier::is_initialized() {
        if let Ok(genre_results) = genre_classifier::classify(&bliss_samples, 22050, 3) {
            meta.genres = genre_classifier::to_metadata_format(&genre_results);
        }
    }

    Ok((meta, analysis))
}

/// Process audio file from pre-loaded memory buffer
/// This avoids disk I/O during parallel processing phase
pub fn process_file_from_memory(
    path: &Path,
    file_data: Vec<u8>,
    _args: &ScanArgs,
) -> Result<(TrackMetadata, Option<Vec<f32>>)> {
    let start_total = std::time::Instant::now();
    let filename = path.file_name().unwrap_or_default().to_string_lossy();

    // Decode audio from memory buffer (clone data since we need it for both decode and tags)
    let t0 = std::time::Instant::now();
    let decoded = audio_decoder::decode_audio_from_memory(file_data.clone(), path)
        .context("Failed to decode audio from memory")?;
    println!("[TIMING] {}: Decode: {:?}", filename, t0.elapsed());

    // Compute fingerprint from decoded samples
    let t1 = std::time::Instant::now();
    let fp = fingerprint::compute_fingerprint_from_decoded(&decoded)
        .context("Fingerprint generation failed")?;
    println!("[TIMING] {}: Fingerprint: {:?}", filename, t1.elapsed());

    let duration = decoded.duration_secs;

    // Build metadata - now from memory!
    let t2 = std::time::Instant::now();
    let mut meta = organizer::read_tags_from_memory(&file_data, path)
        .unwrap_or_else(|_| organizer::TrackMetadata::default());
    meta.duration = duration;
    meta.fingerprint = Some(fp.clone());

    println!("[TIMING] {}: Metadata/Lookup: {:?}", filename, t2.elapsed());

    // Melody Analysis (Bliss) - now from memory using Song::analyze
    let t3 = std::time::Instant::now();
    let bliss_samples = decoded.to_bliss_samples();
    let analysis = match bliss_audio::Song::analyze(&bliss_samples) {
        Ok(bliss_analysis) => Some(bliss_analysis.as_vec()),
        Err(_e) => None,
    };
    println!("[TIMING] {}: Bliss Analysis: {:?}", filename, t3.elapsed());

    // Genre Classification (if models are loaded)
    // Use bliss_samples (mono f32 @ 22050Hz)
    let t4 = std::time::Instant::now();
    if genre_classifier::is_initialized() {
        if let Ok(genre_results) = genre_classifier::classify(&bliss_samples, 22050, 3) {
            meta.genres = genre_classifier::to_metadata_format(&genre_results);
        }
    }
    println!(
        "[TIMING] {}: Genre Classification: {:?}",
        filename,
        t4.elapsed()
    );

    println!(
        "[TIMING] {}: TOTAL PROCESS: {:?}",
        filename,
        start_total.elapsed()
    );

    Ok((meta, analysis))
}
