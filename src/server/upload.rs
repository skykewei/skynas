use crate::models::{TaskStatus, UploadTask};
use crate::server::AppState;
use crate::websocket::WsEvent;
use axum::{
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::time::Instant;
use tokio::io::AsyncWriteExt;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InitUploadRequest {
    pub filename: String,
    pub album: String,
    pub total_size: i64,
    pub total_chunks: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InitUploadResponse {
    pub upload_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UploadChunkQuery {
    pub upload_id: String,
    pub chunk_index: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UploadStatus {
    pub upload_id: String,
    pub received_chunks: i32,
    pub total_chunks: i32,
    pub complete: bool,
}

#[instrument(skip(state, req), fields(filename = %req.filename, album = %req.album))]
pub async fn init_upload(
    State(state): State<AppState>,
    Json(req): Json<InitUploadRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let start = Instant::now();
    let upload_id = Uuid::new_v4().to_string();

    info!(
        upload_id = %upload_id,
        filename = %req.filename,
        album = %req.album,
        total_size = req.total_size,
        total_chunks = req.total_chunks,
        "Upload session initialized"
    );

    // Create cancellation token and store it
    let cancel_token = CancellationToken::new();
    state
        .active_uploads
        .lock()
        .await
        .insert(upload_id.clone(), cancel_token.clone());

    // Create upload task record
    let task = UploadTask {
        id: upload_id.clone(),
        filename: req.filename.clone(),
        album: req.album.clone(),
        total_bytes: req.total_size,
        received_bytes: 0,
        status: TaskStatus::Pending,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        cancelled: false,
    };

    {
        let db = state.db.lock().await;
        db.create_upload_task(&task).map_err(|e| {
            error!(upload_id = %upload_id, error = %e, "Failed to create upload task");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    }

    let temp_path = state
        .config
        .storage
        .base_path
        .join(".temp")
        .join(&upload_id);

    // Create temp directory
    debug!(upload_id = %upload_id, temp_path = %temp_path.display(), "Creating temp directory");
    tokio::fs::create_dir_all(&temp_path)
        .await
        .map_err(|e| {
            error!(upload_id = %upload_id, error = %e, "Failed to create temp directory");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Record in database
    debug!(upload_id = %upload_id, "Recording upload session in database");
    {
        let db = state.db.lock().await;
        db.create_upload_session(
            &upload_id,
            &req.filename,
            &req.album,
            req.total_size,
            req.total_chunks,
            &temp_path.to_string_lossy(),
        )
        .map_err(|e| {
            error!(upload_id = %upload_id, error = %e, "Failed to create upload session in database");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    }

    // Send WebSocket event
    let ws_event = WsEvent::UploadStarted {
        upload_id: upload_id.clone(),
        filename: req.filename.clone(),
        album: req.album.clone(),
        total_bytes: req.total_size,
        total_chunks: req.total_chunks,
    };
    let _ = state.event_sender.send(ws_event);

    let elapsed = start.elapsed().as_millis();
    info!(
        upload_id = %upload_id,
        elapsed_ms = elapsed,
        "Upload session ready for chunks"
    );

    Ok(Json(InitUploadResponse { upload_id }))
}

#[instrument(skip(state, multipart), fields(upload_id = %query.upload_id, chunk_index = query.chunk_index))]
pub async fn upload_chunk(
    State(state): State<AppState>,
    Query(query): Query<UploadChunkQuery>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, StatusCode> {
    let start = Instant::now();

    debug!(upload_id = %query.upload_id, chunk_index = query.chunk_index, "Receiving chunk");

    // Check for cancellation
    if let Some(token) = state.active_uploads.lock().await.get(&query.upload_id)
        && token.is_cancelled()
    {
        warn!(upload_id = %query.upload_id, chunk_index = query.chunk_index, "Upload has been cancelled");
        return Err(StatusCode::REQUEST_TIMEOUT);
    }

    // Get upload session
    let session = {
        let db = state.db.lock().await;
        db.get_upload_session(&query.upload_id)
            .map_err(|e| {
                error!(upload_id = %query.upload_id, error = %e, "Database error getting upload session");
                StatusCode::INTERNAL_SERVER_ERROR
            })?
    };

    let session = session.ok_or_else(|| {
        warn!(upload_id = %query.upload_id, "Upload session not found");
        // Send error event
        let _ = state.event_sender.send(WsEvent::UploadError {
            upload_id: query.upload_id.clone(),
            filename: "unknown".to_string(),
            error: "Upload session not found".to_string(),
            stage: "chunk".to_string(),
        });
        StatusCode::NOT_FOUND
    })?;

    // Update task status to uploading
    {
        let db = state.db.lock().await;
        if let Ok(Some(mut task)) = db.get_upload_task(&query.upload_id) &&
            matches!(task.status, TaskStatus::Pending) {
            task.status = TaskStatus::Uploading;
            task.updated_at = chrono::Utc::now();
            let _ = db.create_upload_task(&task);
        }
    }

    // Receive chunk data
    let mut chunk_data: Option<Vec<u8>> = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| {
            error!(upload_id = %query.upload_id, chunk_index = query.chunk_index, error = %e, "Failed to read multipart field");
            StatusCode::BAD_REQUEST
        })?
    {
        // Check for cancellation during chunk reception
        if let Some(token) = state.active_uploads.lock().await.get(&query.upload_id) {
            if token.is_cancelled() {
                warn!(upload_id = %query.upload_id, chunk_index = query.chunk_index, "Upload cancelled during chunk reception");

                // Update task status to cancelled
                let db = state.db.lock().await;
                if let Ok(Some(mut task)) = db.get_upload_task(&query.upload_id) {
                    task.status = TaskStatus::Cancelled;
                    task.updated_at = chrono::Utc::now();
                    let _ = db.create_upload_task(&task);
                }

                // Send cancellation event
                let _ = state.event_sender.send(WsEvent::UploadError {
                    upload_id: query.upload_id.clone(),
                    filename: "unknown".to_string(),
                    error: "Upload cancelled by user".to_string(),
                    stage: "cancelled".to_string(),
                });

                // Remove from active uploads
                state.active_uploads.lock().await.remove(&query.upload_id);

                return Err(StatusCode::REQUEST_TIMEOUT);
            }
        }

        if field.name() == Some("chunk") {
            let data = field.bytes().await.map_err(|e| {
                error!(upload_id = %query.upload_id, chunk_index = query.chunk_index, error = %e, "Failed to read chunk bytes");
                StatusCode::BAD_REQUEST
            })?;
            chunk_data = Some(data.to_vec());
        }
    }

    let chunk_data = chunk_data.ok_or_else(|| {
        error!(upload_id = %query.upload_id, chunk_index = query.chunk_index, "No chunk data in request");
        StatusCode::BAD_REQUEST
    })?;

    let chunk_size = chunk_data.len();
    debug!(upload_id = %query.upload_id, chunk_index = query.chunk_index, chunk_size = chunk_size, "Chunk received");

    // Save chunk to temp file
    let chunk_path =
        std::path::Path::new(&session.temp_path).join(format!("chunk_{}", query.chunk_index));

    let mut file = tokio::fs::File::create(&chunk_path)
        .await
        .map_err(|e| {
            error!(upload_id = %query.upload_id, chunk_index = query.chunk_index, path = %chunk_path.display(), error = %e, "Failed to create chunk file");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    file.write_all(&chunk_data)
        .await
        .map_err(|e| {
            error!(upload_id = %query.upload_id, chunk_index = query.chunk_index, error = %e, "Failed to write chunk data");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    debug!(upload_id = %query.upload_id, chunk_index = query.chunk_index, path = %chunk_path.display(), "Chunk saved to disk");

    // Update progress
    let received_chunks = query.chunk_index + 1;
    let completed = received_chunks >= session.total_chunks;

    let received_bytes = if completed {
        session.total_size
    } else {
        // Estimate: use actual bytes from saved chunks
        let chunk_files = std::fs::read_dir(&session.temp_path)
            .ok()
            .map(|entries| {
                entries.filter_map(|e| e.ok())
                    .filter(|e| e.file_name().to_string_lossy().starts_with("chunk_"))
                    .count() as i64 * chunk_size as i64
            })
            .unwrap_or(0);
        chunk_files.min(session.total_size)
    };

    let percent = ((received_bytes as f64 / session.total_size as f64) * 100.0) as u8;

    {
        let db = state.db.lock().await;
        db.update_upload_progress(
            &query.upload_id,
            query.chunk_index,
            received_bytes,
            completed,
        )
        .map_err(|e| {
            error!(upload_id = %query.upload_id, chunk_index = query.chunk_index, error = %e, "Failed to update upload progress in database");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    }

    // Send WebSocket events
    let chunk_event = WsEvent::ChunkReceived {
        upload_id: query.upload_id.clone(),
        chunk_index: query.chunk_index,
        total_chunks: session.total_chunks,
    };
    let _ = state.event_sender.send(chunk_event);

    let progress_event = WsEvent::UploadProgress {
        upload_id: query.upload_id.clone(),
        filename: session.filename.clone(),
        received_bytes,
        total_bytes: session.total_size,
        percent,
    };
    let _ = state.event_sender.send(progress_event);

    let elapsed = start.elapsed().as_millis();
    info!(
        upload_id = %query.upload_id,
        chunk_index = query.chunk_index,
        total_chunks = session.total_chunks,
        received_chunks = received_chunks,
        percent = percent,
        chunk_size = chunk_size,
        elapsed_ms = elapsed,
        "Chunk processed"
    );

    Ok(Json(UploadStatus {
        upload_id: query.upload_id,
        received_chunks,
        total_chunks: session.total_chunks,
        complete: completed,
    }))
}

#[instrument(skip(state), fields(upload_id = %upload_id))]
pub async fn complete_upload(
    State(state): State<AppState>,
    Path(upload_id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let start = Instant::now();
    info!(upload_id = %upload_id, "Starting upload completion");

    // Check for cancellation
    if let Some(token) = state.active_uploads.lock().await.get(&upload_id)
        && token.is_cancelled()
    {
        warn!(upload_id = %upload_id, "Upload has been cancelled");

        // Clean up session
        let db = state.db.lock().await;
        let _ = db.delete_upload_session(&upload_id);

        // Remove from active uploads
        state.active_uploads.lock().await.remove(&upload_id);

        return Err(StatusCode::REQUEST_TIMEOUT);
    }

    // Get upload session
    let session = {
        let db = state.db.lock().await;
        db.get_upload_session(&upload_id)
            .map_err(|e| {
                error!(upload_id = %upload_id, error = %e, "Database error getting upload session");
                StatusCode::INTERNAL_SERVER_ERROR
            })?
    };

    let session = session.ok_or_else(|| {
        warn!(upload_id = %upload_id, "Upload session not found during completion");
        // Send error event
        let _ = state.event_sender.send(WsEvent::UploadError {
            upload_id: upload_id.clone(),
            filename: "unknown".to_string(),
            error: "Upload session not found during completion".to_string(),
            stage: "complete".to_string(),
        });
        StatusCode::NOT_FOUND
    })?;

    info!(
        upload_id = %upload_id,
        filename = %session.filename,
        album = %session.album,
        total_chunks = session.total_chunks,
        "Upload session retrieved, beginning merge"
    );

    // Send WebSocket event - starting merge
    let _ = state.event_sender.send(WsEvent::ChunksMerging {
        upload_id: upload_id.clone(),
        filename: session.filename.clone(),
    });

    // Combine chunks
    let album_path = state.config.storage.base_path.join(&session.album);
    debug!(upload_id = %upload_id, album_path = %album_path.display(), "Creating album directory");
    tokio::fs::create_dir_all(&album_path)
        .await
        .map_err(|e| {
            error!(upload_id = %upload_id, album_path = %album_path.display(), error = %e, "Failed to create album directory");
            // Send error event
            let _ = state.event_sender.send(WsEvent::UploadError {
                upload_id: upload_id.clone(),
                filename: session.filename.clone(),
                error: format!("Failed to create album directory: {}", e),
                stage: "merge".to_string(),
            });
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let final_path = album_path.join(&session.filename);
    debug!(upload_id = %upload_id, final_path = %final_path.display(), "Creating final file");
    let mut final_file = tokio::fs::File::create(&final_path)
        .await
        .map_err(|e| {
            error!(upload_id = %upload_id, final_path = %final_path.display(), error = %e, "Failed to create final file");
            let _ = state.event_sender.send(WsEvent::UploadError {
                upload_id: upload_id.clone(),
                filename: session.filename.clone(),
                error: format!("Failed to create final file: {}", e),
                stage: "merge".to_string(),
            });
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let merge_start = Instant::now();
    for i in 0..session.total_chunks {
        let chunk_path = std::path::Path::new(&session.temp_path).join(format!("chunk_{}", i));
        debug!(upload_id = %upload_id, chunk_index = i, chunk_path = %chunk_path.display(), "Reading chunk");

        let chunk_data = tokio::fs::read(&chunk_path)
            .await
            .map_err(|e| {
                error!(upload_id = %upload_id, chunk_index = i, chunk_path = %chunk_path.display(), error = %e, "Failed to read chunk");
                let _ = state.event_sender.send(WsEvent::UploadError {
                    upload_id: upload_id.clone(),
                    filename: session.filename.clone(),
                    error: format!("Failed to read chunk {}: {}", i, e),
                    stage: "merge".to_string(),
                });
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        final_file
            .write_all(&chunk_data)
            .await
            .map_err(|e| {
                error!(upload_id = %upload_id, chunk_index = i, error = %e, "Failed to write chunk to final file");
                let _ = state.event_sender.send(WsEvent::UploadError {
                    upload_id: upload_id.clone(),
                    filename: session.filename.clone(),
                    error: format!("Failed to write chunk {}: {}", i, e),
                    stage: "merge".to_string(),
                });
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        // Delete chunk
        let _ = tokio::fs::remove_file(&chunk_path).await;
    }
    let merge_elapsed = merge_start.elapsed().as_millis();
    info!(upload_id = %upload_id, total_chunks = session.total_chunks, elapsed_ms = merge_elapsed, "All chunks merged");

    // Clean up temp directory
    let _ = tokio::fs::remove_dir_all(&session.temp_path).await;
    debug!(upload_id = %upload_id, temp_path = %session.temp_path, "Temp directory cleaned up");

    // Send WebSocket event - file saved
    let _ = state.event_sender.send(WsEvent::FileSaved {
        upload_id: upload_id.clone(),
        filename: session.filename.clone(),
        path: final_path.to_string_lossy().to_string(),
        size: session.total_size,
    });

    // Calculate hash
    debug!(upload_id = %upload_id, "Calculating file hash");
    let file_data = tokio::fs::read(&final_path)
        .await
        .map_err(|e| {
            error!(upload_id = %upload_id, error = %e, "Failed to read final file for hash");
            let _ = state.event_sender.send(WsEvent::UploadError {
                upload_id: upload_id.clone(),
                filename: session.filename.clone(),
                error: format!("Failed to calculate hash: {}", e),
                stage: "hash".to_string(),
            });
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    let file_hash = format!("{:x}", sha2::Sha256::digest(&file_data));
    debug!(upload_id = %upload_id, hash = %file_hash, "File hash calculated");

    // Try HEIC conversion
    let converter = crate::converter::HeicConverter::new(state.config.heic_converter.clone());

    // Send event - starting conversion
    let _ = state.event_sender.send(WsEvent::HeicConverting {
        upload_id: upload_id.clone(),
        filename: session.filename.clone(),
    });

    let has_jpeg = if let Ok(Some(jpeg_path)) = converter.convert(&final_path) {
        info!(
            upload_id = %upload_id,
            original = %session.filename,
            converted = %jpeg_path.display(),
            "HEIC converted to JPEG"
        );
        // Send conversion success event
        let _ = state.event_sender.send(WsEvent::HeicConverted {
            upload_id: upload_id.clone(),
            original: session.filename.clone(),
            converted: jpeg_path.to_string_lossy().to_string(),
            success: true,
        });
        true
    } else {
        debug!(upload_id = %upload_id, filename = %session.filename, "No HEIC conversion needed or conversion skipped");
        // Send conversion skip event (not an error, just not applicable)
        let _ = state.event_sender.send(WsEvent::HeicConverted {
            upload_id: upload_id.clone(),
            original: session.filename.clone(),
            converted: String::new(),
            success: false,
        });
        false
    };

    // Send event - saving to database
    let _ = state.event_sender.send(WsEvent::DatabaseSaving {
        upload_id: upload_id.clone(),
        filename: session.filename.clone(),
    });

    // Save to database
    debug!(upload_id = %upload_id, "Saving to database");
    {
        let db = state.db.lock().await;
        let photo = crate::models::Photo {
            id: 0,
            filename: session.filename.clone(),
            album: session.album.clone(),
            file_hash: Some(file_hash),
            size_bytes: session.total_size,
            created_at: None,
            uploaded_at: chrono::Utc::now(),
            local_path: final_path.to_string_lossy().to_string(),
            has_jpeg_variant: has_jpeg,
            thumbnail_path: None,
            width: None,
            height: None,
        };
        db.insert_photo(&photo)
            .map_err(|e| {
                error!(upload_id = %upload_id, filename = %session.filename, error = %e, "Failed to save photo to database");
                let _ = state.event_sender.send(WsEvent::UploadError {
                    upload_id: upload_id.clone(),
                    filename: session.filename.clone(),
                    error: format!("Database error: {}", e),
                    stage: "database".to_string(),
                });
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
    }
    info!(upload_id = %upload_id, "Photo saved to database");

    // Show notification
    crate::notify::show_upload_complete(1, &session.album);

    // Trigger cloud sync if enabled
    if state.config.sync.auto_sync {
        let _ = state.event_sender.send(WsEvent::CloudSyncTriggered {
            upload_id: upload_id.clone(),
            filename: session.filename.clone(),
        });

        info!(upload_id = %upload_id, "Cloud sync triggered");
        let sync_manager = crate::sync::SyncManager::new(state.config.clone());
        #[allow(clippy::let_underscore_future)]
        let _ = sync_manager.spawn_sync_task(state.config.sync.sync_delay_seconds);
    }

    // Send completion event
    let _ = state.event_sender.send(WsEvent::UploadComplete {
        upload_id: upload_id.clone(),
        filename: session.filename.clone(),
        album: session.album.clone(),
        size: session.total_size,
    });

    // Clean up upload session
    {
        let db = state.db.lock().await;
        db.delete_upload_session(&upload_id).ok();
    }

    // Update task status to completed
    {
        let db = state.db.lock().await;
        if let Ok(Some(mut task)) = db.get_upload_task(&upload_id) {
            task.status = TaskStatus::Completed;
            task.updated_at = chrono::Utc::now();
            let _ = db.create_upload_task(&task);
        }
    }

    // Remove from active uploads
    state.active_uploads.lock().await.remove(&upload_id);

    let total_elapsed = start.elapsed().as_millis();
    info!(
        upload_id = %upload_id,
        filename = %session.filename,
        album = %session.album,
        size_bytes = session.total_size,
        total_chunks = session.total_chunks,
        has_jpeg_variant = has_jpeg,
        total_elapsed_ms = total_elapsed,
        "Upload completed successfully"
    );

    Ok(Json(serde_json::json!({
        "success": true,
        "upload_id": upload_id,
        "filename": session.filename,
        "album": session.album,
        "size": session.total_size
    })))
}

#[instrument(skip(state), fields(upload_id = %upload_id))]
pub async fn get_upload_status(
    State(state): State<AppState>,
    Path(upload_id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    debug!(upload_id = %upload_id, "Querying upload status");

    let session = {
        let db = state.db.lock().await;
        db.get_upload_session(&upload_id)
            .map_err(|e| {
                error!(upload_id = %upload_id, error = %e, "Database error getting upload status");
                StatusCode::INTERNAL_SERVER_ERROR
            })?
    };

    if let Some(session) = session {
        let received_chunks = session.chunk_index + 1;
        let percent = ((received_chunks as f64 / session.total_chunks as f64) * 100.0) as u8;

        debug!(
            upload_id = %upload_id,
            filename = %session.filename,
            received_chunks = received_chunks,
            total_chunks = session.total_chunks,
            percent = percent,
            completed = session.completed,
            "Upload status retrieved"
        );

        Ok(Json(UploadStatus {
            upload_id,
            received_chunks,
            total_chunks: session.total_chunks,
            complete: session.completed,
        }))
    } else {
        warn!(upload_id = %upload_id, "Upload status requested but session not found");
        Err(StatusCode::NOT_FOUND)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::websocket::create_event_channel;

    /// Test that WsEvent types are correctly instantiated
    #[test]
    fn test_ws_event_types() {
        let (sender, _receiver) = create_event_channel();

        // Test UploadStarted event
        let started = WsEvent::UploadStarted {
            upload_id: "test-123".to_string(),
            filename: "photo.jpg".to_string(),
            album: "test-album".to_string(),
            total_bytes: 1024,
            total_chunks: 10,
        };
        assert!(sender.send(started).is_ok());

        // Test UploadProgress event
        let progress = WsEvent::UploadProgress {
            upload_id: "test-123".to_string(),
            filename: "photo.jpg".to_string(),
            received_bytes: 512,
            total_bytes: 1024,
            percent: 50,
        };
        assert!(sender.send(progress).is_ok());

        // Test ChunkReceived event
        let chunk = WsEvent::ChunkReceived {
            upload_id: "test-123".to_string(),
            chunk_index: 5,
            total_chunks: 10,
        };
        assert!(sender.send(chunk).is_ok());

        // Test UploadComplete event
        let complete = WsEvent::UploadComplete {
            upload_id: "test-123".to_string(),
            filename: "photo.jpg".to_string(),
            album: "test-album".to_string(),
            size: 1024,
        };
        assert!(sender.send(complete).is_ok());

        // Test UploadError event
        let error = WsEvent::UploadError {
            upload_id: "test-123".to_string(),
            filename: "photo.jpg".to_string(),
            error: "Test error".to_string(),
            stage: "test".to_string(),
        };
        assert!(sender.send(error).is_ok());
    }

    /// Test percent calculation for progress tracking
    #[test]
    fn test_percent_calculation() {
        let total_bytes: i64 = 1000;
        let received_bytes: i64 = 500;
        let percent = ((received_bytes as f64 / total_bytes as f64) * 100.0) as u8;
        assert_eq!(percent, 50);

        let received_bytes: i64 = 0;
        let percent = ((received_bytes as f64 / total_bytes as f64) * 100.0) as u8;
        assert_eq!(percent, 0);

        let received_bytes: i64 = 1000;
        let percent = ((received_bytes as f64 / total_bytes as f64) * 100.0) as u8;
        assert_eq!(percent, 100);
    }

    /// Test request/response struct serialization
    #[test]
    fn test_upload_struct_serialization() {
        let init_request = InitUploadRequest {
            filename: "test.jpg".to_string(),
            album: "vacation".to_string(),
            total_size: 1024000,
            total_chunks: 10,
        };

        let json = serde_json::to_string(&init_request).unwrap();
        assert!(json.contains("test.jpg"));
        assert!(json.contains("vacation"));

        let response = InitUploadResponse {
            upload_id: "uuid-123".to_string(),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("uuid-123"));

        let status = UploadStatus {
            upload_id: "uuid-123".to_string(),
            received_chunks: 5,
            total_chunks: 10,
            complete: false,
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("uuid-123"));
        assert!(json.contains("false"));
    }
}
