//! Unified Audio Decoder Module
//!
//! Decodes audio files once and provides PCM data for both fingerprinting and analysis.
//! This eliminates duplicate file reads/decodes that were causing high disk I/O.
//!
//! Optimization: Files are read entirely into memory before decoding to reduce
//! random disk I/O and disk head seeking on HDDs.

use anyhow::{Context, Result};
use std::fs;
use std::io::Cursor;
use std::path::Path;

use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// Holds decoded audio data in multiple formats for different consumers
pub struct DecodedAudio {
    /// Interleaved i16 samples for chromaprint fingerprinting
    pub samples_i16: Vec<i16>,
    /// Sample rate of the audio
    pub sample_rate: u32,
    /// Number of channels
    pub channels: u32,
    /// Duration in seconds
    pub duration_secs: f64,
}

impl DecodedAudio {
    /// Convert samples to bliss-audio format: mono, f32, 22050 Hz
    ///
    /// This allows using Song::analyze() directly from memory without disk I/O.
    pub fn to_bliss_samples(&self) -> Vec<f32> {
        const BLISS_SAMPLE_RATE: u32 = 22050;

        // Step 1: Convert to mono by averaging channels
        let mono_samples: Vec<f32> = if self.channels == 1 {
            self.samples_i16
                .iter()
                .map(|&s| s as f32 / 32768.0)
                .collect()
        } else {
            // Average channels to mono
            self.samples_i16
                .chunks(self.channels as usize)
                .map(|chunk| {
                    let sum: f32 = chunk.iter().map(|&s| s as f32).sum();
                    sum / (self.channels as f32 * 32768.0)
                })
                .collect()
        };

        // Step 2: Resample to 22050 Hz if needed
        if self.sample_rate == BLISS_SAMPLE_RATE {
            mono_samples
        } else {
            // Simple linear interpolation resampling
            let ratio = self.sample_rate as f64 / BLISS_SAMPLE_RATE as f64;
            let output_len = (mono_samples.len() as f64 / ratio) as usize;
            let mut resampled = Vec::with_capacity(output_len);

            for i in 0..output_len {
                let src_pos = i as f64 * ratio;
                let src_idx = src_pos as usize;
                let frac = (src_pos - src_idx as f64) as f32;

                if src_idx + 1 < mono_samples.len() {
                    // Linear interpolation
                    let sample =
                        mono_samples[src_idx] * (1.0 - frac) + mono_samples[src_idx + 1] * frac;
                    resampled.push(sample);
                } else if src_idx < mono_samples.len() {
                    resampled.push(mono_samples[src_idx]);
                }
            }
            resampled
        }
    }
}

/// Decode an audio file into PCM samples
///
/// Uses symphonia to decode once and provide data for both fingerprinting and analysis.
/// The file is first read entirely into memory to reduce random disk I/O.
pub fn decode_audio(path: &Path) -> Result<DecodedAudio> {
    // Read entire file into memory first - this makes disk access sequential
    // and avoids repeated seeks during decoding
    let file_data = fs::read(path).context("Failed to read audio file into memory")?;
    let cursor = Cursor::new(file_data);
    let mss = MediaSourceStream::new(Box::new(cursor), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .context("Failed to probe audio format")?;

    let mut format = probed.format;

    // Find the first audio track
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .context("No audio track found")?;

    let track_id = track.id;

    // Get sample rate and channels
    let sample_rate = track
        .codec_params
        .sample_rate
        .context("No sample rate in track")?;
    let channels = track
        .codec_params
        .channels
        .map(|c| c.count() as u32)
        .unwrap_or(2);

    // Calculate duration if available
    let duration_secs = track
        .codec_params
        .n_frames
        .map(|frames| frames as f64 / sample_rate as f64)
        .unwrap_or(0.0);

    // Create decoder
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .context("Failed to create decoder")?;

    let mut samples_i16 = Vec::new();

    // Decode all packets
    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break; // End of stream
            }
            Err(e) => return Err(e.into()),
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
            Err(e) => return Err(e.into()),
        };

        // Convert to i16 samples
        convert_to_i16(&decoded, &mut samples_i16);
    }

    // Recalculate duration from actual samples if we didn't get it from metadata
    let actual_duration = if duration_secs == 0.0 && sample_rate > 0 && channels > 0 {
        (samples_i16.len() as f64) / (sample_rate as f64 * channels as f64)
    } else {
        duration_secs
    };

    Ok(DecodedAudio {
        samples_i16,
        sample_rate,
        channels,
        duration_secs: actual_duration,
    })
}

/// Decode audio from a pre-loaded memory buffer
///
/// This is used for batch preloading - files are read into memory first,
/// then decoded in parallel without disk I/O.
pub fn decode_audio_from_memory(file_data: Vec<u8>, path: &Path) -> Result<DecodedAudio> {
    let cursor = Cursor::new(file_data);
    let mss = MediaSourceStream::new(Box::new(cursor), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .context("Failed to probe audio format from memory")?;

    let mut format = probed.format;

    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .context("No audio track found")?;

    let track_id = track.id;
    let sample_rate = track
        .codec_params
        .sample_rate
        .context("No sample rate in track")?;
    let channels = track
        .codec_params
        .channels
        .map(|c| c.count() as u32)
        .unwrap_or(2);

    let duration_secs = track
        .codec_params
        .n_frames
        .map(|frames| frames as f64 / sample_rate as f64)
        .unwrap_or(0.0);

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .context("Failed to create decoder")?;

    let mut samples_i16 = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(e) => return Err(e.into()),
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
            Err(e) => return Err(e.into()),
        };

        convert_to_i16(&decoded, &mut samples_i16);
    }

    let actual_duration = if duration_secs == 0.0 && sample_rate > 0 && channels > 0 {
        (samples_i16.len() as f64) / (sample_rate as f64 * channels as f64)
    } else {
        duration_secs
    };

    Ok(DecodedAudio {
        samples_i16,
        sample_rate,
        channels,
        duration_secs: actual_duration,
    })
}

/// Convert decoded audio buffer to interleaved i16 samples
fn convert_to_i16(buffer: &AudioBufferRef, output: &mut Vec<i16>) {
    match buffer {
        AudioBufferRef::S16(buf) => {
            // Already i16, just copy
            let planes = buf.planes();
            let num_channels = planes.planes().len();
            let num_frames = buf.frames();

            if num_channels == 1 {
                output.extend_from_slice(planes.planes()[0]);
            } else {
                // Interleave channels
                for frame in 0..num_frames {
                    for ch in 0..num_channels {
                        output.push(planes.planes()[ch][frame]);
                    }
                }
            }
        }
        AudioBufferRef::S32(buf) => {
            let planes = buf.planes();
            let num_channels = planes.planes().len();
            let num_frames = buf.frames();

            for frame in 0..num_frames {
                for ch in 0..num_channels {
                    // Convert i32 to i16 (shift right 16 bits)
                    let sample = (planes.planes()[ch][frame] >> 16) as i16;
                    output.push(sample);
                }
            }
        }
        AudioBufferRef::F32(buf) => {
            let planes = buf.planes();
            let num_channels = planes.planes().len();
            let num_frames = buf.frames();

            for frame in 0..num_frames {
                for ch in 0..num_channels {
                    // Convert f32 [-1.0, 1.0] to i16
                    let sample =
                        (planes.planes()[ch][frame] * 32767.0).clamp(-32768.0, 32767.0) as i16;
                    output.push(sample);
                }
            }
        }
        AudioBufferRef::F64(buf) => {
            let planes = buf.planes();
            let num_channels = planes.planes().len();
            let num_frames = buf.frames();

            for frame in 0..num_frames {
                for ch in 0..num_channels {
                    let sample =
                        (planes.planes()[ch][frame] * 32767.0).clamp(-32768.0, 32767.0) as i16;
                    output.push(sample);
                }
            }
        }
        AudioBufferRef::U8(buf) => {
            let planes = buf.planes();
            let num_channels = planes.planes().len();
            let num_frames = buf.frames();

            for frame in 0..num_frames {
                for ch in 0..num_channels {
                    // Convert u8 [0, 255] to i16
                    let sample = ((planes.planes()[ch][frame] as i16) - 128) * 256;
                    output.push(sample);
                }
            }
        }
        AudioBufferRef::U16(buf) => {
            let planes = buf.planes();
            let num_channels = planes.planes().len();
            let num_frames = buf.frames();

            for frame in 0..num_frames {
                for ch in 0..num_channels {
                    // Convert u16 to i16
                    let sample = (planes.planes()[ch][frame] as i32 - 32768) as i16;
                    output.push(sample);
                }
            }
        }
        AudioBufferRef::U24(buf) => {
            let planes = buf.planes();
            let num_channels = planes.planes().len();
            let num_frames = buf.frames();

            for frame in 0..num_frames {
                for ch in 0..num_channels {
                    let val = planes.planes()[ch][frame].inner();
                    let sample = ((val >> 8) as i32 - 32768) as i16;
                    output.push(sample);
                }
            }
        }
        AudioBufferRef::U32(buf) => {
            let planes = buf.planes();
            let num_channels = planes.planes().len();
            let num_frames = buf.frames();

            for frame in 0..num_frames {
                for ch in 0..num_channels {
                    let val = planes.planes()[ch][frame];
                    let sample = ((val >> 16) as i32 - 32768) as i16;
                    output.push(sample);
                }
            }
        }
        AudioBufferRef::S24(buf) => {
            let planes = buf.planes();
            let num_channels = planes.planes().len();
            let num_frames = buf.frames();

            for frame in 0..num_frames {
                for ch in 0..num_channels {
                    let val = planes.planes()[ch][frame].inner();
                    let sample = (val >> 8) as i16;
                    output.push(sample);
                }
            }
        }
        AudioBufferRef::S8(buf) => {
            let planes = buf.planes();
            let num_channels = planes.planes().len();
            let num_frames = buf.frames();

            for frame in 0..num_frames {
                for ch in 0..num_channels {
                    // Convert i8 to i16 (shift left 8 bits)
                    let sample = (planes.planes()[ch][frame] as i16) * 256;
                    output.push(sample);
                }
            }
        }
    }
}
