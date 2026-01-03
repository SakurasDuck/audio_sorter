use crate::storage::{AudioLibrary, IndexedTrack};
use crate::TrackMetadata;
use anyhow::{Context, Result};
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use sysinfo::{Disks, System};

#[derive(Debug, Clone, serde::Serialize)]
pub struct ResourceStats {
    pub cpu_usage: f32,
    pub memory_usage: u64, // in bytes
    pub disk_usage: u64,   // in bytes (used space on target drive)
    pub disk_total: u64,   // in bytes (total space on target drive)
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ScanProgress {
    pub is_scanning: bool,
    pub files_total: usize,
    pub files_processed: usize,
    pub current_file: String,
    pub elapsed_secs: u64,
    pub resources: ResourceStats,
    pub errors: usize,
}

impl Default for ScanProgress {
    fn default() -> Self {
        Self {
            is_scanning: false,
            files_total: 0,
            files_processed: 0,
            current_file: String::new(),
            elapsed_secs: 0,
            resources: ResourceStats {
                cpu_usage: 0.0,
                memory_usage: 0,
                disk_usage: 0,
                disk_total: 0,
            },
            errors: 0,
        }
    }
}

pub struct ScanManager {
    progress: Arc<RwLock<ScanProgress>>,
}

impl ScanManager {
    pub fn new() -> Self {
        Self {
            progress: Arc::new(RwLock::new(ScanProgress::default())),
        }
    }

    pub fn get_progress(&self) -> ScanProgress {
        self.progress.read().unwrap().clone()
    }

    pub fn start_scan(
        &self,
        input_dir: PathBuf,
        index_dir: PathBuf,
        offline: bool,
        client_id: Option<String>,
    ) -> Result<()> {
        let progress = self.progress.clone();

        // Check if already scanning
        if progress.read().unwrap().is_scanning {
            return Err(anyhow::anyhow!("Scan already in progress"));
        }

        // Reset progress
        {
            let mut p = progress.write().unwrap();
            *p = ScanProgress::default();
            p.is_scanning = true;
        }

        let index_dir_clone = index_dir.clone();
        tokio::spawn(async move {
            let start_time = Instant::now();
            let progress_for_monitor = progress.clone();
            let monitor_index_dir = index_dir_clone.clone();

            // Start resource monitoring in a separate OS thread (not tokio task)
            // This avoids blocking the tokio runtime with sysinfo calls
            let monitor_handle = std::thread::spawn(move || {
                let mut sys = System::new_all();
                sys.refresh_all();

                let mut disk_usage = 0u64;
                let mut disk_total = 0u64;
                let mut disk_refresh_counter = 0u32;

                loop {
                    std::thread::sleep(Duration::from_millis(500));

                    // Check if scan finished
                    let is_scanning = match progress_for_monitor.try_read() {
                        Ok(p) => p.is_scanning,
                        Err(_) => true,
                    };

                    if !is_scanning {
                        break;
                    }

                    // Refresh system info
                    sys.refresh_cpu_usage();
                    sys.refresh_memory();

                    let cpu_usage = sys.cpus().iter().map(|c| c.cpu_usage()).sum::<f32>()
                        / sys.cpus().len().max(1) as f32;

                    // Refresh disk info every 10 iterations (5 seconds)
                    disk_refresh_counter += 1;
                    if disk_refresh_counter >= 10 {
                        disk_refresh_counter = 0;
                        let disks = Disks::new_with_refreshed_list();
                        if let Some(d) = disks
                            .iter()
                            .find(|d| monitor_index_dir.starts_with(d.mount_point()))
                        {
                            disk_usage = d.total_space() - d.available_space();
                            disk_total = d.total_space();
                        }
                    }

                    if let Ok(mut p) = progress_for_monitor.try_write() {
                        p.elapsed_secs = start_time.elapsed().as_secs();
                        p.resources.cpu_usage = cpu_usage;
                        p.resources.memory_usage = sys.used_memory();
                        p.resources.disk_usage = disk_usage;
                        p.resources.disk_total = disk_total;
                    }
                }
            });

            // Run actual scan in a blocking thread
            let scan_progress = progress.clone();
            let scan_result = tokio::task::spawn_blocking(move || {
                Self::run_scan_logic(input_dir, index_dir, offline, client_id, scan_progress)
            })
            .await;

            // Cleanup
            {
                let mut p = progress.write().unwrap();
                p.is_scanning = false;
                p.elapsed_secs = start_time.elapsed().as_secs();
            }

            // Wait for monitor thread to finish
            let _ = monitor_handle.join();

            if let Err(e) = scan_result {
                eprintln!("Scan task failed: {:?}", e);
            } else if let Ok(Err(e)) = scan_result {
                eprintln!("Scan failed: {}", e);
            }
        });

        Ok(())
    }

    fn run_scan_logic(
        input_dir: PathBuf,
        index_dir: PathBuf,
        offline: bool,
        client_id: Option<String>,
        progress: Arc<RwLock<ScanProgress>>,
    ) -> Result<()> {
        let index_path = index_dir.join("index.json");
        let analysis_path = index_dir.join("analysis.bin");

        // 1. Load Index
        let mut library = AudioLibrary::load(&index_path).unwrap_or_default();
        let mut analysis_store =
            crate::analysis_store::AnalysisStore::load(&analysis_path).unwrap_or_default();

        // 2. Scan Directory
        let files = crate::scanner::scan_directory(&input_dir)?;

        {
            let mut p = progress.write().unwrap();
            p.files_total = files.len();
        }

        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // 3. Diff Phase
        let mut files_to_process = Vec::new();
        let mut skipped_count = 0;

        for path in &files {
            if let Ok(metadata) = std::fs::metadata(path) {
                let mtime = metadata
                    .modified()
                    .unwrap_or(SystemTime::UNIX_EPOCH)
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let size = metadata.len();

                let needs_update = if let Some(indexed) = library.files.get(path) {
                    if indexed.modified_time != mtime || indexed.file_size != size {
                        true
                    } else {
                        // Check if analysis is missing
                        analysis_store.get(path).is_none()
                    }
                } else {
                    true
                };

                if needs_update {
                    files_to_process.push((path.clone(), size, mtime));
                } else {
                    skipped_count += 1;
                }
            }
        }

        // Auto-fill processed count for skipped files
        {
            let mut p = progress.write().unwrap();
            p.files_processed = skipped_count;
        }

        if files_to_process.is_empty() {
            return Ok(());
        }

        // 4. Process Phase (Parallel)
        // 4. Process Phase (Batched Parallelism)
        let batch_size = 50;
        let mut processed_c = skipped_count;
        let mut error_c = 0;

        // Configure Rayon thread pool to limit concurrency
        // Use logical cores - 1, minimum 1 to prevent UI freeze
        let num_threads = std::cmp::max(
            1,
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(2)
                .saturating_sub(1),
        );
        // Also cap at 4 to prevent disk thrashing (high I/O latency)
        let num_threads = std::cmp::min(num_threads, 4);
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build()
            .unwrap();

        pool.install(|| {
            for chunk in files_to_process.chunks(batch_size) {
                // Process chunk in parallel
                let chunk_results: Vec<(
                    PathBuf,
                    u64,
                    u64,
                    Result<(TrackMetadata, Option<Vec<f32>>)>,
                )> = chunk
                    .par_iter()
                    .map_init(
                        || reqwest::blocking::Client::new(),
                        |client, (path, size, mtime)| {
                            let args = crate::ScanArgs {
                                input_dir: input_dir.clone(),
                                output_dir: index_dir.clone(),
                                offline,
                                client_id: client_id.clone(),
                            };

                            let result = crate::worker::process_file(path, &args, client);
                            (path.clone(), *size, *mtime, result)
                        },
                    )
                    .collect();

                // Merge results (Single-threaded to avoid lock contention on library/store)
                for (path, size, mtime, result) in chunk_results {
                    processed_c += 1;
                    match result {
                        Ok((meta, analysis_opt)) => {
                            let entry = IndexedTrack {
                                path: path.clone(),
                                file_size: size,
                                modified_time: mtime,
                                scanned_at: current_time,
                                metadata: meta,
                            };
                            library.files.insert(path.clone(), entry);

                            if let Some(analysis) = analysis_opt {
                                analysis_store.insert(path, analysis);
                            }
                        }
                        Err(_) => {
                            // Only log error, don't stop scan
                            // eprintln!("Error: {}", e);
                            error_c += 1;
                        }
                    }
                }

                // Update Progress (Once per batch)
                if let Ok(mut p) = progress.write() {
                    p.files_processed = processed_c;
                    p.errors = error_c;
                    // Update current file to show activity (using last file of the batch)
                    if let Some(last) = chunk.last() {
                        if let Some(name) = last.0.file_name().and_then(|s| s.to_str()) {
                            p.current_file = name.to_string();
                        }
                    }
                }

                // Periodic Save (Every 4 batches = 200 files)
                if processed_c % 200 == 0 {
                    let _ = library.save(&index_path);
                    let _ = analysis_store.save(&analysis_path);
                }
            }
        });

        // 6. Save Index
        library.save(&index_path)?;
        analysis_store.save(&analysis_path)?;

        Ok(())
    }
}
