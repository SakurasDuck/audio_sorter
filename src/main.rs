use anyhow::Result;
use clap::{Parser, Subcommand};
use rayon::prelude::*;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub mod acoustid;
pub mod analysis_store;
pub mod audio_decoder;
pub mod cache;
pub mod fingerprint;
pub mod genre_classifier;
pub mod html_template;
pub mod musicbrainz;
pub mod organizer;
pub mod recommend;
pub mod scan_manager;
pub mod scanner;
pub mod server;
pub mod storage;
pub mod worker;

use organizer::TrackMetadata;
use storage::{AudioLibrary, IndexedTrack};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Scan directory and update index
    Scan(ScanArgs),
    /// Start web dashboard
    Serve(ServeArgs),
    /// Run genre classification
    Classify(ClassifyArgs),
}

#[derive(Parser, Debug)]
pub struct ScanArgs {
    /// Input directory to scan
    #[arg(short, long)]
    input_dir: PathBuf,

    /// Directory to store index data (index.json)
    #[arg(short, long)]
    output_dir: PathBuf,

    /// Offline mode (skip AcoustID/MusicBrainz and only use local tags)
    #[arg(long, default_value_t = false)]
    offline: bool,

    /// AcoustID Client ID (Optional in offline mode)
    #[arg(long, env = "ACOUSTID_CLIENT_ID")]
    client_id: Option<String>,
}

#[derive(Parser, Debug)]
pub struct ClassifyArgs {
    /// Directory containing index data (index.json) where library is stored
    #[arg(short, long)]
    index_dir: PathBuf,

    /// Directory containing ONNX models (optional, defaults to assets/models)
    #[arg(short, long)]
    model_dir: Option<PathBuf>,
}

#[derive(Parser, Debug)]
struct ServeArgs {
    /// Directory containing index data (index.json)
    #[arg(long)]
    index_dir: PathBuf,

    /// Port to listen on
    #[arg(long, default_value_t = 3000)]
    port: u16,

    /// Input directory to scan (required for web-based scanning)
    #[arg(long)]
    input_dir: Option<PathBuf>,

    /// Directory containing ONNX models (optional)
    #[arg(long)]
    model_dir: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    let cli = Cli::parse();

    match cli.command {
        Commands::Scan(args) => run_scan(args).await,
        Commands::Serve(args) => run_serve(args).await,
        Commands::Classify(args) => run_classify(args).await,
    }
}

async fn run_serve(args: ServeArgs) -> Result<()> {
    server::start_server(args.index_dir, args.input_dir, args.model_dir, args.port).await;
    Ok(())
}

async fn run_classify(args: ClassifyArgs) -> Result<()> {
    let model_dir = args
        .model_dir
        .unwrap_or_else(|| PathBuf::from("assets/models"));

    if !model_dir.exists() {
        eprintln!("Error: Model directory {:?} does not exist.", model_dir);
        return Ok(());
    }

    let manager = scan_manager::ScanManager::new();
    println!("Starting genre classification...");
    println!("Index: {:?}", args.index_dir);
    println!("Models: {:?}", model_dir);

    manager.start_classify(args.index_dir, model_dir)?;

    // Poll for completion
    loop {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        let p = manager.get_progress();

        if !p.is_scanning {
            if p.errors > 0 {
                println!("Finished with {} errors.", p.errors);
            } else {
                println!("Finished successfully.");
            }
            break;
        }
        print!(
            "\rProcessed: {} files... (CPU: {:.1}%, MEM: {} MB)",
            p.files_processed,
            p.resources.cpu_usage,
            p.resources.memory_usage / 1024 / 1024
        );
        use std::io::Write;
        std::io::stdout().flush().ok();
    }
    println!();
    Ok(())
}

async fn run_scan(args: ScanArgs) -> Result<()> {
    // Note: Scanning is CPU heavy, but we are running inside tokio main now.
    // Ideally we should use spawn_blocking for Rayon, but for a simplified CLI tool it's okay-ish
    // provided we don't block the async runtime too badly if we had other web tasks (which we don't during scan).
    // Actually, let's keep it simple. Rayon manages its own thread pool.

    println!("Starting Audio Sorter - Multi-threaded Indexer");
    println!("Input: {:?}", args.input_dir);
    println!("Index Dir: {:?}", args.output_dir);
    if args.offline {
        println!("Mode: OFFLINE");
    } else {
        println!("Mode: ONLINE");
    }

    // 1. Load Index
    let index_path = args.output_dir.join("index.json");
    let analysis_path = args.output_dir.join("analysis.bin");

    let mut library = match AudioLibrary::load(&index_path) {
        Ok(lib) => {
            println!("Loaded existing index with {} entries.", lib.files.len());
            lib
        }
        Err(e) => {
            eprintln!("Could not load existing index: {}. Starting fresh.", e);
            AudioLibrary::default()
        }
    };

    let mut analysis_store = match analysis_store::AnalysisStore::load(&analysis_path) {
        Ok(store) => {
            println!(
                "Loaded existing analysis store with {} entries.",
                store.data.len()
            );
            store
        }
        Err(e) => {
            eprintln!("Could not load analysis store: {}. Starting fresh.", e);
            analysis_store::AnalysisStore::default()
        }
    };

    // 2. Scan Directory
    println!("Scanning directory...");
    let files = scanner::scan_directory(&args.input_dir)?;
    println!("Found {} candidate files.", files.len());

    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // 3. Diff Phase (Serial)
    println!("Identifying changed files...");
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
                    // Check if analysis is missing (e.g. added later)
                    if analysis_store.get(path).is_none() {
                        true
                    } else {
                        false
                    }
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

    let to_process_count = files_to_process.len();
    println!(
        "Skipped {} unchanged files. Processing {} new/modified files...",
        skipped_count, to_process_count
    );

    if to_process_count == 0 {
        println!("Nothing to do.");
        return Ok(());
    }

    // 4. Process Phase (Parallel)
    // Rayon uses its own thread pool, safe to call from here.
    let processed_results: Vec<(PathBuf, u64, u64, Result<(TrackMetadata, Option<Vec<f32>>)>)> =
        files_to_process
            .par_iter()
            .map(|(path, size, mtime)| {
                let result = worker::process_file(path, &args);
                (path.clone(), *size, *mtime, result)
            })
            .collect();

    // 5. Merge Phase
    let mut success_count = 0;
    let mut error_count = 0;

    for (path, size, mtime, result) in processed_results {
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

                success_count += 1;
            }
            Err(e) => {
                eprintln!("Error processing {:?}: {}", path, e);
                error_count += 1;
            }
        }
    }

    // 6. Save Index
    println!("\nScan complete.");
    println!("Processed: {}, Errors: {}", success_count, error_count);
    println!("Saving index to {:?}...", index_path);
    library.save(&index_path)?;
    println!("Saving analysis store to {:?}...", analysis_path);
    analysis_store.save(&analysis_path)?;
    println!("Done!");

    Ok(())
}
