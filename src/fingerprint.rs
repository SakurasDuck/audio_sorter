//! Audio Fingerprinting Module
//!
//! Uses rusty-chromaprint for native fingerprint generation.
//! Replaces the previous fpcalc external process approach.

use anyhow::Result;
use rusty_chromaprint::{Configuration, FingerprintCompressor, Fingerprinter};
use std::path::Path;

use crate::audio_decoder::{self, DecodedAudio};

/// Compute audio fingerprint from a file path (legacy interface)
///
/// This decodes the file internally. For better performance, use
/// `compute_fingerprint_from_decoded` with pre-decoded audio.
pub fn compute_fingerprint(path: &Path) -> Result<(f64, String)> {
    let decoded = audio_decoder::decode_audio(path)?;
    let fp = compute_fingerprint_from_decoded(&decoded)?;
    Ok((decoded.duration_secs, fp))
}

/// Compute fingerprint from pre-decoded audio samples
///
/// This avoids re-decoding when audio is already decoded for other purposes.
pub fn compute_fingerprint_from_decoded(decoded: &DecodedAudio) -> Result<String> {
    compute_fingerprint_from_samples(&decoded.samples_i16, decoded.sample_rate, decoded.channels)
}

/// Compute fingerprint from raw PCM samples
///
/// # Arguments
/// * `samples` - Interleaved i16 PCM samples
/// * `sample_rate` - Sample rate in Hz
/// * `channels` - Number of audio channels
pub fn compute_fingerprint_from_samples(
    samples: &[i16],
    sample_rate: u32,
    channels: u32,
) -> Result<String> {
    if samples.is_empty() {
        return Err(anyhow::anyhow!("No audio samples provided"));
    }

    // Use preset_test2 which is compatible with AcoustID
    let config = Configuration::preset_test2();
    let mut printer = Fingerprinter::new(&config);

    printer
        .start(sample_rate, channels)
        .map_err(|e| anyhow::anyhow!("Failed to start fingerprinter: {:?}", e))?;

    printer.consume(samples);
    printer.finish();

    let raw_fp = printer.fingerprint();
    if raw_fp.is_empty() {
        return Err(anyhow::anyhow!("No fingerprint generated"));
    }

    // Compress and encode the fingerprint (same format as fpcalc output)
    let compressor = FingerprintCompressor::from(&config);
    let compressed = compressor.compress(raw_fp);
    let encoded = base64_encode(&compressed);

    Ok(encoded)
}

/// Base64 encode bytes (URL-safe variant used by Chromaprint)
fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

    let mut result = String::new();
    let mut buffer = 0u32;
    let mut bits = 0;

    for &byte in data {
        buffer = (buffer << 8) | (byte as u32);
        bits += 8;

        while bits >= 6 {
            bits -= 6;
            let idx = ((buffer >> bits) & 0x3F) as usize;
            result.push(ALPHABET[idx] as char);
        }
    }

    if bits > 0 {
        buffer <<= 6 - bits;
        let idx = (buffer & 0x3F) as usize;
        result.push(ALPHABET[idx] as char);
    }

    result
}
