# Melody Fingerprinting and Recommendation Implementation Plan

# Goal Description
Enable the application to analyze audio files for their "melody" features (timbre, tempo, etc.) and provide similarity-based recommendations using `bliss-audio` (with `symphonia` backend for Windows compatibility).

## Status: ✅ Implementation Complete

## Implemented Changes

### Dependencies
#### [MODIFY] [Cargo.toml](file:///f:/code/rust_test/audio-sorter/Cargo.toml)
-   Added `bliss-audio = { version = "0.11", features = ["aubio-static", "symphonia-all", "serde"], default-features = false }`
-   Added `bincode = "1.3"` for efficient binary serialization.

---

### Data Structures
#### [NEW] [src/analysis_store.rs](file:///f:/code/rust_test/audio-sorter/src/analysis_store.rs)
-   Created `AnalysisStore` struct to manage `HashMap<PathBuf, Vec<f32>>`.
-   Implements `load()` / `save()` using `bincode` for high-performance binary I/O.
-   Storage file: `analysis.bin` (sibling to `index.json`).

---

### Core Logic
#### [MODIFY] [src/worker.rs](file:///f:/code/rust_test/audio-sorter/src/worker.rs)
-   Updated `process_file` to return `Result<(TrackMetadata, Option<Vec<f32>>)>`.
-   Integrated `SymphoniaDecoder::song_from_path()` for melody analysis.
-   Converts `Analysis` to `Vec<f32>` via `as_vec()`.

#### [MODIFY] [src/main.rs](file:///f:/code/rust_test/audio-sorter/src/main.rs)
-   Added `analysis_store` module registration.
-   Loads/saves `AnalysisStore` alongside `AudioLibrary`.
-   Diff phase now also checks for missing analysis data.

#### [MODIFY] [src/scan_manager.rs](file:///f:/code/rust_test/audio-sorter/src/scan_manager.rs)
-   Integrated `AnalysisStore` loading/saving in web-triggered scans.
-   Updated type annotations for new worker return type.

---

### API & Web Interface
#### [MODIFY] [src/server.rs](file:///f:/code/rust_test/audio-sorter/src/server.rs)
-   Added `GET /api/recommend?path=...` endpoint.
-   Implemented `euclidean_distance()` function for similarity calculation.
-   Returns top 20 similar songs with distance scores.

---

## Verification Plan

### Manual Verification
1.  **Scan**: Run `cargo run -- scan -i <audio_dir> -o <data_dir>` on test samples.
2.  **Verify Files**: Check that `analysis.bin` is created alongside `index.json`.
3.  **API Test**: Start server and call `/api/recommend?path=<song_path>` to verify recommendations.

### Build Verification
-   ✅ `cargo check` passed with warnings only
-   ✅ `cargo build --release` completed successfully (1m 41s)
