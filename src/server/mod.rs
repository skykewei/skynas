use crate::config::Config;
use crate::db::Database;
use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    response::{Html, IntoResponse, Json},
    routing::{get, post},
    Router,
};
use sha2::Digest;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

mod upload;
use upload::{complete_upload, get_upload_status, init_upload, upload_chunk};

pub struct AppState {
    pub config: Config,
    pub db: Arc<Mutex<Database>>,
}

impl Clone for AppState {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            db: Arc::clone(&self.db),
        }
    }
}

pub async fn run_server(config: Config, db: Database) -> anyhow::Result<()> {
    let state = AppState {
        config: config.clone(),
        db: Arc::new(Mutex::new(db)),
    };

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/api/upload", post(upload_handler))
        .route("/api/upload/chunked/init", post(init_upload))
        .route("/api/upload/chunked/chunk", post(upload_chunk))
        .route("/api/upload/chunked/complete/:upload_id", post(complete_upload))
        .route("/api/upload/chunked/status/:upload_id", get(get_upload_status))
        .route("/api/health", get(health_handler))
        .nest_service("/static", ServeDir::new("src/server/static"))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    println!("Server listening on http://{}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}

async fn index_handler() -> impl IntoResponse {
    let html = include_str!("static/index.html");
    Html(html)
}

async fn health_handler() -> impl IntoResponse {
    Json(serde_json::json!({"status": "ok"}))
}

async fn upload_handler(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, StatusCode> {
    let mut album = String::from("未分类");
    let mut file_data: Option<(String, Vec<u8>)> = None;

    while let Some(field) = multipart.next_field().await.map_err(|_| StatusCode::BAD_REQUEST)? {
        let name = field.name().unwrap_or("").to_string();

        match name.as_str() {
            "album" => {
                album = field.text().await.unwrap_or_default();
            }
            "file" => {
                let filename = field.file_name().unwrap_or("unknown").to_string();
                let data = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;
                file_data = Some((filename, data.to_vec()));
            }
            _ => {}
        }
    }

    if let Some((filename, data)) = file_data {
        // Save file
        let album_path = state.config.storage.base_path.join(&album);
        tokio::fs::create_dir_all(&album_path).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let file_path = album_path.join(&filename);
        tokio::fs::write(&file_path, &data).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        // Record in database
        let file_hash = format!("{:x}", sha2::Sha256::digest(&data));
        let size = data.len() as i64;

        let photo = crate::models::Photo {
            id: 0,
            filename: filename.clone(),
            album: album.clone(),
            file_hash: Some(file_hash),
            size_bytes: size,
            created_at: None,
            uploaded_at: chrono::Utc::now(),
            local_path: file_path.to_string_lossy().to_string(),
            has_jpeg_variant: false,
        };

        {
            let db = state.db.lock().await;
            if let Err(e) = db.insert_photo(&photo) {
                eprintln!("Failed to record photo in database: {}", e);
            }
        }

        Ok(Json(serde_json::json!({
            "success": true,
            "filename": filename,
            "album": album,
            "size": size
        })))
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}
