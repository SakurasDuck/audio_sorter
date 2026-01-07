use anyhow::{Context, Result};
use std::path::Path;

use crate::acoustid;
use crate::audio_decoder;
use crate::fingerprint;
use crate::musicbrainz;
use crate::organizer::{self, TrackMetadata};
use crate::ScanArgs;

// Import decoder trait and implementation for bliss analysis
use bliss_audio::decoder::symphonia::SymphoniaDecoder;
use bliss_audio::decoder::Decoder as DecoderTrait;

pub fn process_file(
    path: &Path,
    args: &ScanArgs,
    client: &reqwest::blocking::Client,
) -> Result<(TrackMetadata, Option<Vec<f32>>)> {
    // Decode audio once using our unified decoder
    let decoded = audio_decoder::decode_audio(path).context("Failed to decode audio file")?;

    // Compute fingerprint from decoded samples (no re-reading file)
    let fp = fingerprint::compute_fingerprint_from_decoded(&decoded)
        .context("Fingerprint generation failed")?;

    let duration = decoded.duration_secs;

    // Build metadata
    let meta = if args.offline || args.client_id.is_none() {
        let mut meta = organizer::read_tags(path).context("Failed to read local tags")?;
        meta.duration = duration;
        meta.fingerprint = Some(fp.clone());
        meta
    } else {
        match perform_online_lookup(args, client, duration, &fp) {
            Ok(meta) => meta,
            Err(_e) => {
                let mut meta = organizer::read_tags(path)?;
                meta.duration = duration;
                meta.fingerprint = Some(fp.clone());
                meta
            }
        }
    };

    // Melody Analysis (Bliss) - still uses its own decoder for now
    // TODO: In future, could modify bliss to accept pre-decoded samples
    let analysis = match SymphoniaDecoder::song_from_path(path) {
        Ok(song) => {
            // Convert Analysis to Vec<f32>
            Some(song.analysis.as_vec())
        }
        Err(_e) => None,
    };

    Ok((meta, analysis))
}

/// Process audio file from pre-loaded memory buffer
/// This avoids disk I/O during parallel processing phase
pub fn process_file_from_memory(
    path: &Path,
    file_data: Vec<u8>,
    args: &ScanArgs,
    client: &reqwest::blocking::Client,
) -> Result<(TrackMetadata, Option<Vec<f32>>)> {
    // Decode audio from memory buffer (clone data since we need it for both decode and tags)
    let decoded = audio_decoder::decode_audio_from_memory(file_data.clone(), path)
        .context("Failed to decode audio from memory")?;

    // Compute fingerprint from decoded samples
    let fp = fingerprint::compute_fingerprint_from_decoded(&decoded)
        .context("Fingerprint generation failed")?;

    let duration = decoded.duration_secs;

    // Build metadata - now from memory!
    let meta = if args.offline || args.client_id.is_none() {
        let mut meta = organizer::read_tags_from_memory(&file_data, path)
            .unwrap_or_else(|_| organizer::TrackMetadata::default());
        meta.duration = duration;
        meta.fingerprint = Some(fp.clone());
        meta
    } else {
        match perform_online_lookup(args, client, duration, &fp) {
            Ok(meta) => meta,
            Err(_e) => {
                let mut meta = organizer::read_tags_from_memory(&file_data, path)
                    .unwrap_or_else(|_| organizer::TrackMetadata::default());
                meta.duration = duration;
                meta.fingerprint = Some(fp.clone());
                meta
            }
        }
    };

    // Melody Analysis (Bliss) - now from memory using Song::analyze
    let bliss_samples = decoded.to_bliss_samples();
    let analysis = match bliss_audio::Song::analyze(&bliss_samples) {
        Ok(bliss_analysis) => Some(bliss_analysis.as_vec()),
        Err(_e) => None,
    };

    Ok((meta, analysis))
}

fn perform_online_lookup(
    args: &ScanArgs,
    client: &reqwest::blocking::Client,
    duration: f64,
    fp: &str,
) -> Result<TrackMetadata> {
    let client_id = args
        .client_id
        .as_ref()
        .context("No Client ID provided for online lookup")?;

    let lookup =
        acoustid::lookup_fingerprint(client_id, duration, fp).context("AcoustID lookup failed")?;

    if let Some(results) = lookup.results {
        if let Some(best_match) = results.first() {
            if let Some(recordings) = &best_match.recordings {
                if let Some(recording) = recordings.first() {
                    let rec_id = &recording.id;
                    let title = recording.title.as_deref().unwrap_or("Unknown Title");
                    let artist = recording
                        .artists
                        .as_ref()
                        .and_then(|a| a.first())
                        .map(|a| a.name.as_str())
                        .unwrap_or("Unknown Artist");

                    let final_artist = artist.to_string();
                    let final_title = title.to_string();
                    let mut original_artist = None;
                    let mut original_title = None;
                    let album = None; // Metadata from AcoustID is limited, usually need MB lookups for album

                    match musicbrainz::fetch_recording_details(client, rec_id) {
                        Ok(mb_rec) => {
                            if let Some(rels) = mb_rec.relations {
                                for rel in rels {
                                    if let Some(work) = rel.work {
                                        if let Ok(work_data) =
                                            musicbrainz::fetch_work_recordings(client, &work.id)
                                        {
                                            if let Some(work_rels) = work_data.relations {
                                                for wr in work_rels {
                                                    if let Some(rec) = wr.recording {
                                                        if let Some(credits) = rec.artist_credit {
                                                            if let Some(first_credit) =
                                                                credits.first()
                                                            {
                                                                if first_credit.name != final_artist
                                                                {
                                                                    original_artist = Some(
                                                                        first_credit.name.clone(),
                                                                    );
                                                                    original_title =
                                                                        Some(rec.title.clone());
                                                                    break;
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Err(_) => {}
                    }

                    return Ok(TrackMetadata {
                        title: final_title,
                        artist: final_artist,
                        album,
                        original_artist,
                        original_title,
                        duration,
                        fingerprint: Some(fp.to_string()),
                    });
                }
            }
        }
    }

    Err(anyhow::anyhow!("No valid match found online"))
}
