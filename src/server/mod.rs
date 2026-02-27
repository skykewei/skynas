use crate::config::Config;
use crate::db::Database;
use crate::websocket::{EventSender, WsEvent, create_event_channel, ws_handler};
use axum::{
    Router,
    extract::{DefaultBodyLimit, Multipart, State},
    http::StatusCode,
    response::{Html, IntoResponse, Json},
    routing::{get, post},
};
use sha2::Digest;
use std::sync::Arc;
use std::time::Instant;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

mod upload;
use upload::{complete_upload, get_upload_status, init_upload, upload_chunk};

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub db: Arc<Mutex<Database>>,
    pub event_sender: EventSender,
}

pub async fn run_server(config: Config, db: Database) -> anyhow::Result<()> {
    let (event_sender, _) = create_event_channel();

    let state = AppState {
        config: config.clone(),
        db: Arc::new(Mutex::new(db)),
        event_sender,
    };

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/ws", get(ws_handler))
        .route("/api/upload", post(upload_handler))
        .route("/api/upload/chunked/init", post(init_upload))
        .route("/api/upload/chunked/chunk", post(upload_chunk))
        .route(
            "/api/upload/chunked/complete/:upload_id",
            post(complete_upload),
        )
        .route(
            "/api/upload/chunked/status/:upload_id",
            get(get_upload_status),
        )
        .route("/api/health", get(health_handler))
        .nest_service("/static", ServeDir::new("src/server/static"))
        .layer(CorsLayer::permissive())
        .layer(DefaultBodyLimit::max(100 * 1024 * 1024)) // 100MB limit for streaming
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

#[instrument(skip(state, multipart))]
async fn upload_handler(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, StatusCode> {
    let start = Instant::now();
    let upload_id = Uuid::new_v4().to_string();

    let mut album = String::from("未分类");
    let mut file_info: Option<(String, String, u64)> = None; // (filename, temp_path, size)

    info!(upload_id = %upload_id, "Starting streaming upload");

    // Create temp directory for streaming uploads
    let temp_dir = state
        .config
        .storage
        .base_path
        .join(".temp")
        .join(&upload_id);
    tokio::fs::create_dir_all(&temp_dir)
        .await
        .map_err(|e| {
            error!(upload_id = %upload_id, error = %e, "Failed to create temp directory");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| {
            error!(upload_id = %upload_id, error = %e, "Failed to read multipart field");
            StatusCode::BAD_REQUEST
        })?
    {
        let name = field.name().unwrap_or("").to_string();

        match name.as_str() {
            "album" => {
                album = field.text().await.unwrap_or_default();
                debug!(upload_id = %upload_id, album = %album, "Album selected");
            }
            "file" => {
                let filename = field.file_name().unwrap_or("unknown").to_string();
                debug!(upload_id = %upload_id, filename = %filename, "Receiving file data (streaming)");

                // Create temp file for streaming write
                let temp_path = temp_dir.join(&filename);
                let mut file = tokio::fs::File::create(&temp_path)
                    .await
                    .map_err(|e| {
                        error!(upload_id = %upload_id, temp_path = %temp_path.display(), error = %e, "Failed to create temp file");
                        StatusCode::INTERNAL_SERVER_ERROR
                    })?;

                // Stream data chunks to file
                let mut total_size: u64 = 0;
                let mut stream = field;

                while let Some(chunk) = stream.chunk().await.map_err(|e| {
                    error!(upload_id = %upload_id, filename = %filename, error = %e, "Failed to read file chunk");
                    StatusCode::BAD_REQUEST
                })? {
                    file.write_all(&chunk).await.map_err(|e| {
                        error!(upload_id = %upload_id, filename = %filename, error = %e, "Failed to write file chunk");
                        StatusCode::INTERNAL_SERVER_ERROR
                    })?;
                    total_size += chunk.len() as u64;
                }

                // Flush to ensure all data is written
                file.flush().await.map_err(|e| {
                    error!(upload_id = %upload_id, filename = %filename, error = %e, "Failed to flush file");
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

                drop(file); // Close file handle

                info!(upload_id = %upload_id, filename = %filename, size_bytes = total_size, "File streamed to temp location");
                file_info = Some((filename, temp_path.to_string_lossy().to_string(), total_size));
            }
            _ => {}
        }
    }

    if let Some((filename, temp_path, size)) = file_info {
        let size_i64 = size as i64;

        info!(
            upload_id = %upload_id,
            filename = %filename,
            album = %album,
            size_bytes = size,
            "Processing streamed upload"
        );

        // Send WebSocket event - upload started
        let _ = state.event_sender.send(WsEvent::UploadStarted {
            upload_id: upload_id.clone(),
            filename: filename.clone(),
            album: album.clone(),
            total_bytes: size_i64,
            total_chunks: 1,
        });

        // Create album directory
        let album_path = state.config.storage.base_path.join(&album);
        debug!(upload_id = %upload_id, album_path = %album_path.display(), "Creating album directory");
        tokio::fs::create_dir_all(&album_path)
            .await
            .map_err(|e| {
                error!(upload_id = %upload_id, album_path = %album_path.display(), error = %e, "Failed to create album directory");
                let _ = state.event_sender.send(WsEvent::UploadError {
                    upload_id: upload_id.clone(),
                    filename: filename.clone(),
                    error: format!("Failed to create directory: {}", e),
                    stage: "save".to_string(),
                });
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        let file_path = album_path.join(&filename);
        debug!(upload_id = %upload_id, file_path = %file_path.display(), "Moving file to final location");

        // Move temp file to final location
        tokio::fs::rename(&temp_path, &file_path)
            .await
            .map_err(|e| {
                error!(upload_id = %upload_id, temp_path = %temp_path, file_path = %file_path.display(), error = %e, "Failed to move file");
                let _ = state.event_sender.send(WsEvent::UploadError {
                    upload_id: upload_id.clone(),
                    filename: filename.clone(),
                    error: format!("Failed to save file: {}", e),
                    stage: "save".to_string(),
                });
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        // Clean up temp directory
        let _ = tokio::fs::remove_dir_all(&temp_dir).await;

        info!(upload_id = %upload_id, file_path = %file_path.display(), "File saved to disk");

        // Send progress event - 50%
        let _ = state.event_sender.send(WsEvent::UploadProgress {
            upload_id: upload_id.clone(),
            filename: filename.clone(),
            received_bytes: size_i64,
            total_bytes: size_i64,
            percent: 50,
        });

        // Calculate hash from file on disk (streaming read)
        debug!(upload_id = %upload_id, "Calculating file hash (streaming)");
        let file_hash = tokio::task::spawn_blocking({
            let file_path = file_path.clone();
            move || {
                use std::io::Read;
                let mut file = std::fs::File::open(&file_path)?;
                let mut hasher = sha2::Sha256::new();
                let mut buffer = [0u8; 8192]; // 8KB buffer for streaming hash
                loop {
                    let n = file.read(&mut buffer)?;
                    if n == 0 {
                        break;
                    }
                    hasher.update(&buffer[..n]);
                }
                Ok::<String, std::io::Error>(format!("{:x}", hasher.finalize()))
            }
        })
        .await
        .map_err(|e| {
            error!(upload_id = %upload_id, error = %e, "Hash calculation task failed");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .map_err(|e| {
            error!(upload_id = %upload_id, error = %e, "Failed to calculate file hash");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        debug!(upload_id = %upload_id, hash = %file_hash, "File hash calculated");

        let photo = crate::models::Photo {
            id: 0,
            filename: filename.clone(),
            album: album.clone(),
            file_hash: Some(file_hash),
            size_bytes: size_i64,
            created_at: None,
            uploaded_at: chrono::Utc::now(),
            local_path: file_path.to_string_lossy().to_string(),
            has_jpeg_variant: false,
        };

        debug!(upload_id = %upload_id, "Saving photo to database");
        {
            let db = state.db.lock().await;
            if let Err(e) = db.insert_photo(&photo) {
                error!(upload_id = %upload_id, filename = %filename, error = %e, "Failed to record photo in database");
                let _ = state.event_sender.send(WsEvent::UploadError {
                    upload_id: upload_id.clone(),
                    filename: filename.clone(),
                    error: format!("Database error: {}", e),
                    stage: "database".to_string(),
                });
            } else {
                info!(upload_id = %upload_id, photo_id = photo.id, "Photo recorded in database");
            }
        }

        // Send progress event - 100%
        let _ = state.event_sender.send(WsEvent::UploadProgress {
            upload_id: upload_id.clone(),
            filename: filename.clone(),
            received_bytes: size_i64,
            total_bytes: size_i64,
            percent: 100,
        });

        // Send completion event
        let _ = state.event_sender.send(WsEvent::UploadComplete {
            upload_id: upload_id.clone(),
            filename: filename.clone(),
            album: album.clone(),
            size: size_i64,
        });

        let elapsed = start.elapsed().as_millis();
        info!(
            upload_id = %upload_id,
            filename = %filename,
            album = %album,
            size_bytes = size,
            elapsed_ms = elapsed,
            "Streaming upload completed successfully"
        );

        Ok(Json(serde_json::json!({
            "success": true,
            "upload_id": upload_id,
            "filename": filename,
            "album": album,
            "size": size_i64
        })))
    } else {
        warn!(upload_id = %upload_id, "No file data in upload request");
        Err(StatusCode::BAD_REQUEST)
    }
}
