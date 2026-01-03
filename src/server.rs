use axum::{
    extract::{self, Query, State},
    response::{Html, IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde_json::json;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;

use crate::html_template::HTML_CONTENT;
use crate::scan_manager::ScanManager;
use crate::storage::{AudioLibrary, IndexedTrack};

fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return f32::NAN;
    }
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f32>()
        .sqrt()
}

struct AppState {
    index_path: PathBuf,
    input_dir: Option<PathBuf>,
    scan_manager: Arc<ScanManager>,
}

pub async fn start_server(index_dir: PathBuf, input_dir: Option<PathBuf>, port: u16) {
    let index_path = index_dir.join("index.json");
    let scan_manager = Arc::new(ScanManager::new());

    let state = Arc::new(AppState {
        index_path,
        input_dir,
        scan_manager,
    });

    let app = Router::new()
        .route("/", get(serve_index))
        .route("/api/tracks", get(serve_tracks))
        .route("/api/scan/start", post(start_scan))
        .route("/api/scan/status", get(get_scan_status))
        .route("/api/duplicates", get(get_duplicates))
        .route("/api/recommend", get(get_recommendations))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("Web Dashboard available at http://{}", addr);

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn serve_index() -> Html<&'static str> {
    Html(HTML_CONTENT)
}

async fn serve_tracks(State(state): State<Arc<AppState>>) -> Json<Vec<IndexedTrack>> {
    match AudioLibrary::load(&state.index_path) {
        Ok(lib) => Json(lib.files.into_values().collect()),
        Err(_) => Json(vec![]),
    }
}

async fn start_scan(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let input_dir = match &state.input_dir {
        Some(d) => d.clone(),
        None => return Json(json!({"error": "No input directory configured"})),
    };

    let index_dir = state.index_path.parent().unwrap().to_path_buf();

    // For simplicity, we hardcode offline=false and no client_id for now,
    // or we could accept them in POST body.
    // Assuming defaults for web scan: Offline=false (if configured?), ClientID?
    // Let's assume offline for now to be safe or try online if env var exists?
    // We'll pass None for client_id and offline=true for safety unless we enhance args.
    // Actually, let's try to be smart. If ACOUSTID_CLIENT_ID env is set, use it.

    let client_id = std::env::var("ACOUSTID_CLIENT_ID").ok();
    let offline = client_id.is_none(); // If no key, force offline

    match state
        .scan_manager
        .start_scan(input_dir, index_dir, offline, client_id)
    {
        Ok(_) => Json(json!({"status": "started"})),
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

async fn get_scan_status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let progress = state.scan_manager.get_progress();
    Json(progress)
}

async fn get_duplicates(State(state): State<Arc<AppState>>) -> Json<Vec<Vec<IndexedTrack>>> {
    match AudioLibrary::load(&state.index_path) {
        Ok(lib) => Json(lib.find_duplicates()),
        Err(_) => Json(vec![]),
    }
}

#[derive(serde::Deserialize)]
struct RecommendParams {
    path: String,
}

async fn get_recommendations(
    State(state): State<Arc<AppState>>,
    Query(params): extract::Query<RecommendParams>,
) -> impl IntoResponse {
    let target_path = PathBuf::from(&params.path);
    // analysis.bin is sibling of index.json
    let analysis_path = state.index_path.parent().unwrap().join("analysis.bin");

    let store = match crate::analysis_store::AnalysisStore::load(&analysis_path) {
        Ok(s) => s,
        Err(_) => return Json(json!({"error": "Failed to load analysis store"})),
    };

    let target_analysis = match store.get(&target_path) {
        Some(a) => a,
        None => return Json(json!({"error": "Target song has no analysis data"})),
    };

    let mut results = Vec::new();

    for (path, analysis) in &store.data {
        if path == &target_path {
            continue;
        }

        let distance = euclidean_distance(target_analysis, analysis);
        if distance.is_nan() {
            continue;
        }
        results.push((path, distance));
    }

    // Sort by distance ASC
    results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    // Top 20
    let top_results: Vec<_> = results.into_iter().take(20).collect();

    // Enrich
    let library = match AudioLibrary::load(&state.index_path) {
        Ok(lib) => lib,
        Err(_) => AudioLibrary::default(),
    };

    let enriched: Vec<_> = top_results
        .iter()
        .map(|(path, dist)| {
            let track = library.files.get(*path);
            let title = track
                .map(|t| t.metadata.title.clone())
                .unwrap_or_else(|| "Unknown".to_string());
            let artist = track
                .map(|t| t.metadata.artist.clone())
                .unwrap_or_else(|| "Unknown".to_string());
            let album = track
                .and_then(|t| t.metadata.album.clone())
                .unwrap_or_else(|| "-".to_string());
            json!({
                "path": path.to_string_lossy(),
                "title": title,
                "artist": artist,
                "album": album,
                "distance": dist
            })
        })
        .collect();

    Json(json!(enriched))
}
