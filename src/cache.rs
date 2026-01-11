//! MusicBrainz API Cache Layer
//!
//! Provides LRU caching for MusicBrainz API responses to avoid redundant
//! network requests and respect rate limits.

use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Mutex;

use crate::musicbrainz::{MBRecordingResponse, MBWorkResponse};

/// Default cache capacity for recordings and works
const CACHE_CAPACITY: usize = 1000;

/// Thread-safe LRU cache for MusicBrainz responses
pub struct MusicBrainzCache {
    recordings: Mutex<LruCache<String, MBRecordingResponse>>,
    works: Mutex<LruCache<String, MBWorkResponse>>,
}

impl MusicBrainzCache {
    /// Create a new cache with default capacity
    pub fn new() -> Self {
        Self {
            recordings: Mutex::new(LruCache::new(NonZeroUsize::new(CACHE_CAPACITY).unwrap())),
            works: Mutex::new(LruCache::new(NonZeroUsize::new(CACHE_CAPACITY).unwrap())),
        }
    }

    /// Get a cached recording response
    pub fn get_recording(&self, id: &str) -> Option<MBRecordingResponse> {
        self.recordings.lock().ok()?.get(id).cloned()
    }

    /// Cache a recording response
    pub fn put_recording(&self, id: String, data: MBRecordingResponse) {
        if let Ok(mut cache) = self.recordings.lock() {
            cache.put(id, data);
        }
    }

    /// Get a cached work response
    pub fn get_work(&self, id: &str) -> Option<MBWorkResponse> {
        self.works.lock().ok()?.get(id).cloned()
    }

    /// Cache a work response
    pub fn put_work(&self, id: String, data: MBWorkResponse) {
        if let Ok(mut cache) = self.works.lock() {
            cache.put(id, data);
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> (usize, usize) {
        let rec_len = self.recordings.lock().map(|c| c.len()).unwrap_or(0);
        let work_len = self.works.lock().map(|c| c.len()).unwrap_or(0);
        (rec_len, work_len)
    }
}

impl Default for MusicBrainzCache {
    fn default() -> Self {
        Self::new()
    }
}
