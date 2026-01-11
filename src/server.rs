use axum::http::HeaderMap;
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
use tower_http::services::ServeDir;

use crate::html_template::HTML_CONTENT;
use crate::scan_manager::ScanManager;
use crate::storage::{AudioLibrary, IndexedTrack};

struct AppState {
    index_path: PathBuf,
    input_dir: Option<PathBuf>,
    model_dir: Option<PathBuf>,
    scan_manager: Arc<ScanManager>,
}

pub async fn start_server(
    index_dir: PathBuf,
    input_dir: Option<PathBuf>,
    model_dir: Option<PathBuf>,
    port: u16,
) {
    let index_path = index_dir.join("index.json");
    let scan_manager = Arc::new(ScanManager::new());

    let state = Arc::new(AppState {
        index_path,
        input_dir: input_dir.clone(),
        model_dir: model_dir.clone(),
        scan_manager,
    });

    let app = Router::new()
        .route("/", get(serve_index))
        .route("/api/tracks", get(serve_tracks))
        .route("/api/scan/start", post(start_scan))
        .route("/api/classify/start", post(start_classify))
        .route("/api/scan/status", get(get_scan_status))
        .route("/api/duplicates", get(get_duplicates))
        .route("/api/recommend", get(get_recommendations))
        .route("/playlist.m3u", get(get_playlist));

    let app = if let Some(dir) = input_dir {
        app.nest_service("/stream", ServeDir::new(dir))
    } else {
        app
    };

    let app = app.with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("Web Dashboard available at http://{}:{}", "127.0.0.1", port);
    println!(
        "Playlist available at http://{}:{}/playlist.m3u",
        "127.0.0.1", port
    );

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

async fn get_playlist(State(state): State<Arc<AppState>>, headers: HeaderMap) -> impl IntoResponse {
    let host = headers
        .get("host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("127.0.0.1");

    let lib = match AudioLibrary::load(&state.index_path) {
        Ok(l) => l,
        Err(_) => {
            return (
                [(
                    axum::http::header::CONTENT_TYPE,
                    "audio/x-mpegurl; charset=utf-8",
                )],
                "#EXTM3U\n# Error: Could not load library".to_string(),
            );
        }
    };

    let mut m3u = String::from("#EXTM3U\n");

    // We need to map file paths to relative paths from the served root.
    // However, AudioLibrary stores absolute paths.
    // If input_dir is set, we can strip the prefix.

    if let Some(root) = &state.input_dir {
        for (path, track) in &lib.files {
            if let Ok(relative) = path.strip_prefix(root) {
                // Determine duration in seconds (integer)
                let duration_secs = track.metadata.duration.round() as i64;

                // Get display title
                let title = if track.metadata.title.is_empty() {
                    "Unknown Title"
                } else {
                    &track.metadata.title
                };
                let artist = if track.metadata.artist.is_empty() {
                    "Unknown Artist"
                } else {
                    &track.metadata.artist
                };

                // EXTINF:duration,Artist - Title
                m3u.push_str(&format!(
                    "#EXTINF:{},{} - {}\n",
                    duration_secs, artist, title
                ));

                // URL: http://<host>/stream/<relative_path>
                // Encode each path segment separately to handle spaces, Chinese chars, etc.
                let url_path: String = relative
                    .iter()
                    .map(|seg| urlencoding::encode(&seg.to_string_lossy()).into_owned())
                    .collect::<Vec<_>>()
                    .join("/");

                let full_url = format!("http://{}/stream/{}", host, url_path);
                m3u.push_str(&full_url);
                m3u.push('\n');
            }
        }
    } else {
        m3u.push_str("# Error: No input directory configured, cannot serve files.");
    }

    // Return with proper Content-Type for M3U playlist
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "audio/x-mpegurl; charset=utf-8",
        )],
        m3u,
    )
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

async fn start_classify(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let input_dir = match &state.input_dir {
        Some(d) => d.clone(),
        None => return Json(json!({"error": "No input directory configured"})),
    };

    let Some(model_dir) = &state.model_dir else {
        return Json(json!({"error": "No model directory configured"}));
    };

    let index_dir = state.index_path.parent().unwrap().to_path_buf();

    match state
        .scan_manager
        .start_classify(index_dir, model_dir.clone())
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
    /// Filter: only include tracks by this artist
    same_artist: Option<String>,
    /// Filter: only include tracks from this album
    same_album: Option<String>,
    /// Filter: exclude tracks from this album
    exclude_album: Option<String>,
    /// Number of results (default 20)
    limit: Option<usize>,
}

async fn get_recommendations(
    State(state): State<Arc<AppState>>,
    Query(params): extract::Query<RecommendParams>,
) -> impl IntoResponse {
    let target_path = PathBuf::from(&params.path);
    let analysis_path = state.index_path.parent().unwrap().join("analysis.bin");

    let store = match crate::analysis_store::AnalysisStore::load(&analysis_path) {
        Ok(s) => s,
        Err(_) => return Json(json!({"error": "Failed to load analysis store"})),
    };

    let library = match AudioLibrary::load(&state.index_path) {
        Ok(lib) => lib,
        Err(_) => return Json(json!({"error": "Failed to load library"})),
    };

    // Get the target track's fingerprint to exclude exact duplicates
    let exclude_fp = library
        .files
        .get(&target_path)
        .and_then(|t| t.metadata.fingerprint.clone());

    // Build filters from query params
    let filters = crate::recommend::RecommendFilters {
        same_artist: params.same_artist,
        same_album: params.same_album,
        exclude_album: params.exclude_album,
        exclude_fingerprint: exclude_fp,
        genre: None,
    };

    let top_k = params.limit.unwrap_or(20);
    let results = crate::recommend::find_similar(&target_path, &library, &store, &filters, top_k);

    let enriched: Vec<_> = results
        .iter()
        .map(|scored| {
            json!({
                "path": scored.track.path.to_string_lossy(),
                "title": scored.track.metadata.title,
                "artist": scored.track.metadata.artist,
                "album": scored.track.metadata.album.clone().unwrap_or_else(|| "-".to_string()),
                "distance": scored.distance
            })
        })
        .collect();

    Json(json!(enriched))
}
