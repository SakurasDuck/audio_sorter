//! Similarity Recommendation Module
//!
//! Provides filtered similarity search based on bliss audio features
//! with optional artist/album metadata filtering.

use crate::analysis_store::AnalysisStore;
use crate::storage::{AudioLibrary, IndexedTrack};
use std::path::Path;

/// Filters for similarity recommendation
#[derive(Debug, Default, Clone)]
pub struct RecommendFilters {
    /// Only include tracks by this artist
    pub same_artist: Option<String>,
    /// Exclude tracks from this album
    pub exclude_album: Option<String>,
    /// Only include tracks from this album
    pub same_album: Option<String>,
    /// Exclude exact duplicates (same fingerprint)
    pub exclude_fingerprint: Option<String>,
    /// Only include tracks with this genre (case-insensitive match)
    pub genre: Option<String>,
}

/// A track with its similarity score (lower = more similar)
#[derive(Debug, Clone)]
pub struct ScoredTrack {
    pub track: IndexedTrack,
    pub distance: f32,
}

/// Compute Euclidean distance between two feature vectors
fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return f32::MAX;
    }
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f32>()
        .sqrt()
}

/// Find similar tracks with optional metadata filtering
///
/// # Arguments
/// * `query_path` - Path to the query track
/// * `library` - Audio library with indexed tracks
/// * `analysis_store` - Bliss analysis data
/// * `filters` - Optional metadata filters
/// * `top_k` - Number of results to return
///
/// # Returns
/// Vector of scored tracks sorted by similarity (most similar first)
pub fn find_similar(
    query_path: &Path,
    library: &AudioLibrary,
    analysis_store: &AnalysisStore,
    filters: &RecommendFilters,
    top_k: usize,
) -> Vec<ScoredTrack> {
    // Get query track features
    let query_features = match analysis_store.get(query_path) {
        Some(f) => f,
        None => return Vec::new(),
    };

    let mut results: Vec<ScoredTrack> = library
        .files
        .values()
        // Exclude the query track itself
        .filter(|track| track.path != query_path)
        // Apply artist filter
        .filter(|track| {
            filters
                .same_artist
                .as_ref()
                .map_or(true, |a| track.metadata.artist.eq_ignore_ascii_case(a))
        })
        // Apply album inclusion filter
        .filter(|track| {
            filters.same_album.as_ref().map_or(true, |a| {
                track
                    .metadata
                    .album
                    .as_ref()
                    .map_or(false, |album| album.eq_ignore_ascii_case(a))
            })
        })
        // Apply album exclusion filter
        .filter(|track| {
            filters.exclude_album.as_ref().map_or(true, |a| {
                track
                    .metadata
                    .album
                    .as_ref()
                    .map_or(true, |album| !album.eq_ignore_ascii_case(a))
            })
        })
        // Exclude exact duplicates by fingerprint
        .filter(|track| {
            filters.exclude_fingerprint.as_ref().map_or(true, |fp| {
                track
                    .metadata
                    .fingerprint
                    .as_ref()
                    .map_or(true, |track_fp| track_fp != fp)
            })
        })
        // Filter by genre (if any of the track's genres match)
        .filter(|track| {
            filters.genre.as_ref().map_or(true, |target_genre| {
                track
                    .metadata
                    .genres
                    .iter()
                    .any(|(label, _conf)| label.eq_ignore_ascii_case(target_genre))
            })
        })
        // Compute similarity score
        .filter_map(|track| {
            let features = analysis_store.get(&track.path)?;
            let distance = euclidean_distance(query_features, features);
            Some(ScoredTrack {
                track: track.clone(),
                distance,
            })
        })
        .collect();

    // Sort by distance (ascending = most similar first)
    results.sort_by(|a, b| {
        a.distance
            .partial_cmp(&b.distance)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Return top K
    results.truncate(top_k);
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_euclidean_distance() {
        let a = vec![0.0, 0.0, 0.0];
        let b = vec![3.0, 4.0, 0.0];
        assert!((euclidean_distance(&a, &b) - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_euclidean_distance_same() {
        let a = vec![1.0, 2.0, 3.0];
        assert!((euclidean_distance(&a, &a) - 0.0).abs() < 0.001);
    }
}
