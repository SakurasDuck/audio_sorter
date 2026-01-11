//! Genre Classification Module
//!
//! Uses MTG-Jamendo 87-genre classifier (EffNet-Discogs).
//!
//! Requires:
//! 1. `onnxruntime` shared library (dll/so/dylib) available at runtime.
//! 2. Model files `discogs-effnet-bs64.onnx` and `mtg_jamendo_genre-discogs-effnet.onnx`.

use anyhow::Result;
#[cfg(feature = "genre-onnx")]
use ndarray::{Array2, Array3, Array4};
#[cfg(feature = "genre-onnx")]
use ort::session::{builder::GraphOptimizationLevel, Session};
#[cfg(feature = "genre-onnx")]
use ort::value::Value;
#[cfg(feature = "genre-onnx")]
use std::cell::RefCell;
#[cfg(feature = "genre-onnx")]
use std::f32::consts::PI;
use std::path::{Path, PathBuf};
#[cfg(feature = "genre-onnx")]
use std::sync::OnceLock;

/// 87 MTG-Jamendo genre labels
pub const GENRE_LABELS: &[&str] = &[
    "60s",
    "70s",
    "80s",
    "90s",
    "acidjazz",
    "alternative",
    "alternativerock",
    "ambient",
    "atmospheric",
    "blues",
    "bluesrock",
    "bossanova",
    "breakbeat",
    "celtic",
    "chanson",
    "chillout",
    "choir",
    "classical",
    "classicrock",
    "club",
    "contemporary",
    "country",
    "dance",
    "darkambient",
    "darkwave",
    "deephouse",
    "disco",
    "downtempo",
    "drumnbass",
    "dub",
    "dubstep",
    "easylistening",
    "edm",
    "electronic",
    "electronica",
    "electropop",
    "ethno",
    "eurodance",
    "experimental",
    "folk",
    "funk",
    "fusion",
    "groove",
    "grunge",
    "hard",
    "hardrock",
    "hiphop",
    "house",
    "idm",
    "improvisation",
    "indie",
    "industrial",
    "instrumentalpop",
    "instrumentalrock",
    "jazz",
    "jazzfusion",
    "latin",
    "lounge",
    "medieval",
    "metal",
    "minimal",
    "newage",
    "newwave",
    "orchestral",
    "pop",
    "popfolk",
    "poprock",
    "postrock",
    "progressive",
    "psychedelic",
    "punkrock",
    "rap",
    "reggae",
    "rnb",
    "rock",
    "rocknroll",
    "singersongwriter",
    "soul",
    "soundtrack",
    "swing",
    "symphonic",
    "synthpop",
    "techno",
    "trance",
    "triphop",
    "world",
    "worldfusion",
];

// DSP Constants for Essentia Models
#[cfg(feature = "genre-onnx")]
const TARGET_SR: usize = 16000;
#[cfg(feature = "genre-onnx")]
const N_FFT: usize = 1024;
#[cfg(feature = "genre-onnx")]
const HOP_LENGTH: usize = 512;
#[cfg(feature = "genre-onnx")]
const N_MELS: usize = 96;
#[cfg(feature = "genre-onnx")]
const PATCH_FRAMES: usize = 128;

/// Global storage for models directory path
#[cfg(feature = "genre-onnx")]
static MODEL_DIR: OnceLock<PathBuf> = OnceLock::new();

#[cfg(feature = "genre-onnx")]
struct ClassifierModels {
    embedding_session: Session,
    classifier_session: Session,
}

#[cfg(feature = "genre-onnx")]
use rubato::{FftFixedIn, Resampler};

// Thread-local storage for Resampler and Models
#[cfg(feature = "genre-onnx")]
thread_local! {
    static RESAMPLER: RefCell<Option<FftFixedIn<f32>>> = RefCell::new(None);
    static MODELS: RefCell<Option<ClassifierModels>> = RefCell::new(None);
}

/// Result of genre classification
#[derive(Debug, Clone)]
pub struct GenreResult {
    pub label: String,
    pub confidence: f32,
}

/// Initialize the genre classifier environment and store model path
pub fn init_models(model_dir: &Path) -> Result<()> {
    #[cfg(not(feature = "genre-onnx"))]
    {
        let _ = model_dir;
        println!("Genre classification disabled (feature 'genre-onnx' not enabled)");
        Ok(())
    }

    #[cfg(feature = "genre-onnx")]
    {
        if MODEL_DIR.get().is_some() {
            return Ok(());
        }

        // Check if models exist
        // Use discogs-effnet-bsdynamic-1.onnx which outputs 1280-dim embeddings
        // (the old discogs-effnet-bs64.onnx only outputs 512-dim which is incompatible)
        let embedding_path = model_dir.join("discogs-effnet-bsdynamic-1.onnx");
        let classifier_path = model_dir.join("mtg_jamendo_genre-discogs-effnet.onnx");

        if !embedding_path.exists() || !classifier_path.exists() {
            println!(
                "Genre models not found at {:?} or {:?}",
                embedding_path, classifier_path
            );
            return Ok(());
        }

        // Initialize ONNX Runtime Environment (global)
        let _ = ort::init().with_name("audio_sorter_classifier").commit();

        // Store model directory for lazy loading in threads
        let _ = MODEL_DIR.set(model_dir.to_path_buf());

        println!("Genre classification initialized. Models will be loaded per-thread.");
        Ok(())
    }
}

/// Check if models are initialized (globally configured)
pub fn is_initialized() -> bool {
    #[cfg(feature = "genre-onnx")]
    {
        MODEL_DIR.get().is_some()
    }
    #[cfg(not(feature = "genre-onnx"))]
    {
        false
    }
}

/// Helper to load models for the current thread
#[cfg(feature = "genre-onnx")]
fn load_thread_models() -> Result<()> {
    MODELS.with(|cell| {
        let mut models = cell.borrow_mut();
        if models.is_some() {
            return Ok(());
        }

        if let Some(model_dir) = MODEL_DIR.get() {
            let embedding_path = model_dir.join("discogs-effnet-bsdynamic-1.onnx");
            let classifier_path = model_dir.join("mtg_jamendo_genre-discogs-effnet.onnx");

            let embedding_session = Session::builder()?
                .with_optimization_level(GraphOptimizationLevel::Level3)?
                .with_intra_threads(1)? // Reduce intra-threads since we run many parallel sessions
                .commit_from_file(&embedding_path)?;
            
            // Print actual input/output names from the ONNX model
            println!("[DEBUG] Embedding model loaded. Inputs:");
            for input in embedding_session.inputs() {
                println!("[DEBUG]   Input: '{}'", input.name());
            }
            println!("[DEBUG] Embedding model outputs:");
            for output in embedding_session.outputs() {
                println!("[DEBUG]   Output: '{}'", output.name());
            }

            let classifier_session = Session::builder()?
                .with_optimization_level(GraphOptimizationLevel::Level3)?
                .with_intra_threads(1)?
                .commit_from_file(&classifier_path)?;
            
            println!("[DEBUG] Classifier model loaded. Inputs:");
            for input in classifier_session.inputs() {
                println!("[DEBUG]   Input: '{}'", input.name());
            }
            println!("[DEBUG] Classifier model outputs:");
            for output in classifier_session.outputs() {
                println!("[DEBUG]   Output: '{}'", output.name());
            }

            *models = Some(ClassifierModels {
                embedding_session,
                classifier_session,
            });
        }
        Ok(())
    })
}

/// Classify audio samples and return top-k genre predictions
pub fn classify(samples: &[f32], sample_rate: u32, top_k: usize) -> Result<Vec<GenreResult>> {
    #[cfg(feature = "genre-onnx")]
    {
        // 0. Ensure models are loaded for this thread
        load_thread_models()?;

        // Access via thread_local
        let mut results = Vec::new(); // Placeholder, we'll assign inside closure

        let start_classify = std::time::Instant::now(); // Timing Log

        MODELS.with(|cell| -> Result<()> {
            let mut borrow = cell.borrow_mut();
            let models = if let Some(m) = borrow.as_mut() {
                m
            } else {
                return Ok(());
            };

            // 1. Resample to 16kHz
            let t0 = std::time::Instant::now();
            let resampled = if sample_rate != TARGET_SR as u32 {
                resample_audio(samples, sample_rate as usize, TARGET_SR)?
            } else {
                samples.to_vec()
            };
            println!("[TIMING] Genre: Resample: {:?}", t0.elapsed());

            // 2. Compute Mel Spectrogram
            let t1 = std::time::Instant::now();
            let mel_spec = compute_log_mel_spectrogram(&resampled)?;
            println!("[TIMING] Genre: Mel Spectrogram: {:?}", t1.elapsed());
            println!("[DEBUG] Genre: Mel spec shape: {} rows x {} cols", mel_spec.nrows(), mel_spec.ncols());

            if mel_spec.nrows() < PATCH_FRAMES {
                println!("[WARN] Genre: Mel spec too short ({} rows < {} required), skipping classification", mel_spec.nrows(), PATCH_FRAMES);
                return Ok(());
            }

            // 3. Create Patches
            let t2 = std::time::Instant::now();
            let patches = create_patches(&mel_spec);
            println!("[TIMING] Genre: Create Patches: {:?}", t2.elapsed());
            println!("[DEBUG] Genre: Created {} patches", patches.len());

            if patches.is_empty() {
                println!("[WARN] Genre: No patches created, skipping classification");
                return Ok(());
            }

            // 4. Run Embedding Model (Batch Processing)
            let t3 = std::time::Instant::now();
            let total_patches = patches.len();
            // ONNX model expects fixed batch size of 64
            const BATCH_SIZE: usize = 64; 
            let mut all_embeddings = Vec::new();

            for chunk in patches.chunks(BATCH_SIZE) {
                // Create input tensor with shape [64, 128, 96] (removing channel dim 1)
                // If chunk is smaller than 64, remaining entries stay 0 (padding)
                let mut input_tensor = Array3::<f32>::zeros((BATCH_SIZE, PATCH_FRAMES, N_MELS));
                
                for (i, patch) in chunk.iter().enumerate() {
                    for r in 0..PATCH_FRAMES {
                        for c in 0..N_MELS {
                            input_tensor[[i, r, c]] = patch[[r, c]];
                        }
                    }
                }

                let shape = input_tensor.shape().to_vec();
                let data = input_tensor.into_raw_vec();
                let input_value = Value::from_array((shape, data))?;
                // ONNX input name for discogs-effnet-bsdynamic-1.onnx
                let inputs = ort::inputs!["melspectrogram" => &input_value];

                // Accessing mutable session here is valid inside current thread!
                let embedding_out = match models.embedding_session.run(inputs) {
                    Ok(out) => out,
                    Err(e) => {
                        eprintln!("[ERROR] Embedding model run failed: {:?}", e);
                        return Err(anyhow::anyhow!("Embedding inference failed: {}", e));
                    }
                };

                // discogs-effnet-bsdynamic-1.onnx has 2 outputs:
                // - "activations" (n, 400) - style predictions
                // - "embeddings" (n, 1280) - embeddings (what we need)
                let embeddings_val = embedding_out.get("embeddings")
                    .ok_or_else(|| anyhow::anyhow!("Missing 'embeddings' output"))?;
                let (embed_shape, embed_data) = embeddings_val.try_extract_tensor::<f32>()?;
                
                let out_batch_size = embed_shape[0] as usize;
                let out_dim = embed_shape[1] as usize; // Should be 1280

                // Only take the valid embeddings corresponding to real patches (ignore padding)
                // For a chunk of size N, we take the first N embeddings
                let valid_count = chunk.len();
                let batch_embeddings_view = ndarray::ArrayView2::from_shape((out_batch_size, out_dim), embed_data)?;
                
                for i in 0..valid_count {
                    for j in 0..out_dim {
                        all_embeddings.push(batch_embeddings_view[[i, j]]);
                    }
                }
            }

            println!("[TIMING] Genre: Embedding Inference (Batched): {:?}", t3.elapsed());
            
            // Reconstruct full embedding matrix
            let total_processed = all_embeddings.len();
            if total_processed == 0 {
                 println!("[WARN] Genre: No embeddings generated");
                 return Ok(());
            }
            
            // Determine embedding dimension from the data we collected
            // If total_patches > 0, we can deduce dim
            let embed_dim = total_processed / total_patches;
            let embed_rows = total_patches;
            let embed_cols = embed_dim;
            
            println!("[DEBUG] Genre: Total Extracted Embeddings shape: {} x {}", embed_rows, embed_cols);

            let embeddings_view = ndarray::ArrayView2::from_shape((embed_rows, embed_cols), &all_embeddings)?;
            println!("[TIMING] Genre: Embedding Inference: {:?}", t3.elapsed());
            println!("[DEBUG] Genre: Embedding shape: {} x {}", embed_rows, embed_cols);

            // 5. Average Embeddings
            let mut track_embedding = Array2::<f32>::zeros((1, embed_cols));
            let num_patches = embed_rows;
            let embed_dim = embed_cols;

            for i in 0..num_patches {
                for j in 0..embed_dim {
                    track_embedding[[0, j]] += embeddings_view[[i, j]];
                }
            }
            for j in 0..embed_dim {
                track_embedding[[0, j]] /= num_patches as f32;
            }

            // 6. Run Classifier Model
            let t4 = std::time::Instant::now();
            
            // L2 normalize the embedding (important for classifier stability)
            let mut norm_sq: f32 = 0.0;
            for j in 0..embed_dim {
                norm_sq += track_embedding[[0, j]] * track_embedding[[0, j]];
            }
            let norm = norm_sq.sqrt();
            if norm > 1e-8 {
                for j in 0..embed_dim {
                    track_embedding[[0, j]] /= norm;
                }
            }
            
            let track_shape = track_embedding.shape().to_vec();
            let track_data = track_embedding.clone().into_raw_vec();
            
            // Debug: Show embedding statistics (after L2 normalization)
            let embed_min = track_data.iter().cloned().fold(f32::INFINITY, f32::min);
            let embed_max = track_data.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            let embed_sum: f32 = track_data.iter().sum();
            let embed_mean = embed_sum / track_data.len() as f32;
            let norm_check: f32 = track_data.iter().map(|x| x * x).sum::<f32>().sqrt();
            println!("[DEBUG] Embedding stats (L2 normalized): min={:.6}, max={:.6}, mean={:.6}, L2_norm={:.6}", 
                embed_min, embed_max, embed_mean, norm_check);
            
            let track_embedding_value = Value::from_array((track_shape, track_data))?;
            // Correct ONNX input name (verified via onnx_debug binary: C_IN=embeddings)
            let classifier_inputs = ort::inputs!["embeddings" => &track_embedding_value];

            let classifier_out = match models.classifier_session.run(classifier_inputs) {
                Ok(out) => out,
                Err(e) => {
                    eprintln!("[ERROR] Classifier model run failed: {:?}", e);
                    return Err(anyhow::anyhow!("Classifier inference failed: {}", e));
                }
            };

            let activations_val = classifier_out.get("activations")
                .ok_or_else(|| anyhow::anyhow!("Missing 'activations' output from classifier"))?;
            let (_act_shape, act_data) = activations_val.try_extract_tensor::<f32>()?;
            println!("[TIMING] Genre: Classifier Inference: {:?}", t4.elapsed());

            // 7. Process Results
            let probs = act_data;
            let mut local_results: Vec<GenreResult> = Vec::new();

            for i in 0..probs.len() {
                local_results.push(GenreResult {
                    label: GENRE_LABELS.get(i).unwrap_or(&"unknown").to_string(),
                    confidence: probs[i],
                });
            }

            local_results.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
            
            // Log top results for debugging
            println!("[DEBUG] Genre: Classification results (top 5):");
            for (i, r) in local_results.iter().take(5).enumerate() {
                println!("[DEBUG]   {}. {} -> {:.4}", i + 1, r.label, r.confidence);
            }
            
            results = local_results;

            println!("[TIMING] Genre: TOTAL: {:?}", start_classify.elapsed());

            Ok(())
        })?;

        Ok(results.into_iter().take(top_k).collect())
    }

    #[cfg(not(feature = "genre-onnx"))]
    {
        let _ = samples;
        let _ = sample_rate;
        let _ = top_k;
        Ok(Vec::new())
    }
}

// DSP Helper Functions (unchanged logic, just context)

#[cfg(feature = "genre-onnx")]
fn resample_audio(samples: &[f32], source_sr: usize, target_sr: usize) -> Result<Vec<f32>> {
    if source_sr == target_sr {
        return Ok(samples.to_vec());
    }

    let chunk_size = 1024;

    RESAMPLER.with(|resampler_cell| {
        let mut borrow = resampler_cell.borrow_mut();

        if borrow.is_none() {
            *borrow = Some(FftFixedIn::<f32>::new(
                source_sr, target_sr, chunk_size, 1, 1,
            )?);
        }

        let resampler = borrow.as_mut().unwrap();

        let mut input_buffer = vec![vec![0.0; chunk_size]; 1];
        let mut output_audio = Vec::with_capacity(
            (samples.len() as f64 * target_sr as f64 / source_sr as f64) as usize,
        );
        let mut pos = 0;

        while pos + chunk_size <= samples.len() {
            input_buffer[0].copy_from_slice(&samples[pos..pos + chunk_size]);
            let waves_out = resampler.process(&input_buffer, None)?;
            output_audio.extend_from_slice(&waves_out[0]);
            pos += chunk_size;
        }

        if pos < samples.len() {
            let remaining = samples.len() - pos;
            let mut last_chunk = vec![0.0; chunk_size];
            last_chunk[..remaining].copy_from_slice(&samples[pos..]);
            input_buffer[0] = last_chunk;
            let waves_out = resampler.process(&input_buffer, None)?;
            output_audio.extend_from_slice(&waves_out[0]);
        }

        Ok(output_audio)
    })
}

#[cfg(feature = "genre-onnx")]
fn compute_log_mel_spectrogram(samples: &[f32]) -> Result<Array2<f32>> {
    use rustfft::{num_complex::Complex, FftPlanner};

    let num_frames = (samples.len() - N_FFT) / HOP_LENGTH + 1;
    if num_frames == 0 {
        return Ok(Array2::zeros((0, N_MELS)));
    }

    let window: Vec<f32> = (0..N_FFT)
        .map(|i| 0.5 * (1.0 - (2.0 * PI * i as f32 / N_FFT as f32).cos()))
        .collect();

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(N_FFT);

    let mel_filters = create_mel_filterbank(TARGET_SR, N_FFT, N_MELS)?;

    let mut spectrogram = Array2::<f32>::zeros((num_frames, N_MELS));
    let mut buffer = vec![Complex { re: 0.0, im: 0.0 }; N_FFT];

    for i in 0..num_frames {
        let start = i * HOP_LENGTH;
        let end = start + N_FFT;
        let frame = &samples[start..end];

        for (j, &s) in frame.iter().enumerate() {
            buffer[j] = Complex {
                re: s * window[j],
                im: 0.0,
            };
        }

        fft.process(&mut buffer);

        let magnitude: Vec<f32> = buffer[..N_FFT / 2 + 1].iter().map(|c| c.norm()).collect();

        for m in 0..N_MELS {
            let mut mel_energy = 0.0;
            for k in 0..magnitude.len() {
                mel_energy += magnitude[k] * mel_filters[[m, k]];
            }
            spectrogram[[i, m]] = (mel_energy + 1e-6).ln();
        }
    }

    Ok(spectrogram)
}

#[cfg(feature = "genre-onnx")]
fn create_mel_filterbank(sr: usize, n_fft: usize, n_mels: usize) -> Result<Array2<f32>> {
    let f_min = 0.0;
    let f_max = sr as f32 / 2.0;

    fn hz_to_mel(hz: f32) -> f32 {
        2595.0 * (1.0 + hz / 700.0).log10()
    }

    fn mel_to_hz(mel: f32) -> f32 {
        700.0 * (10.0f32.powf(mel / 2595.0) - 1.0)
    }

    let mel_min = hz_to_mel(f_min);
    let mel_max = hz_to_mel(f_max);

    let mel_points: Vec<f32> = (0..n_mels + 2)
        .map(|i| mel_min + (mel_max - mel_min) * i as f32 / (n_mels + 1) as f32)
        .collect();

    let hz_points: Vec<f32> = mel_points.iter().map(|&m| mel_to_hz(m)).collect();

    let bin_points: Vec<usize> = hz_points
        .iter()
        .map(|&hz| ((n_fft as f32 + 1.0) * hz / sr as f32).floor() as usize)
        .collect();

    let num_bins = n_fft / 2 + 1;
    let mut filters = Array2::<f32>::zeros((n_mels, num_bins));

    for m in 0..n_mels {
        let left = bin_points[m];
        let center = bin_points[m + 1];
        let right = bin_points[m + 2];

        for k in left..center {
            filters[[m, k]] = (k - left) as f32 / (center - left) as f32;
        }
        for k in center..right {
            filters[[m, k]] = (right - k) as f32 / (right - center) as f32;
        }
    }

    Ok(filters)
}

#[cfg(feature = "genre-onnx")]
fn create_patches(mel_spec: &Array2<f32>) -> Vec<ndarray::ArrayView2<'_, f32>> {
    let mut patches = Vec::new();
    let num_frames = mel_spec.nrows();

    if num_frames < PATCH_FRAMES {
        return patches;
    }

    let stride = PATCH_FRAMES / 2;

    let mut start = 0;
    while start + PATCH_FRAMES <= num_frames {
        let patch = mel_spec.slice(ndarray::s![start..start + PATCH_FRAMES, ..]);
        patches.push(patch);
        start += stride;
    }

    patches
}

/// Convert genre results to the format used in TrackMetadata
pub fn to_metadata_format(results: &[GenreResult]) -> Vec<(String, f32)> {
    results
        .iter()
        .map(|r| (r.label.clone(), r.confidence))
        .collect()
}
