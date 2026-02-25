use crate::server::AppState;
use axum::{
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct InitUploadRequest {
    filename: String,
    album: String,
    total_size: i64,
    total_chunks: i32,
}

#[derive(Debug, Serialize)]
pub struct InitUploadResponse {
    upload_id: String,
}

#[derive(Debug, Deserialize)]
pub struct UploadChunkQuery {
    upload_id: String,
    chunk_index: i32,
}

#[derive(Debug, Serialize)]
pub struct UploadStatus {
    upload_id: String,
    received_chunks: i32,
    total_chunks: i32,
    complete: bool,
}

pub async fn init_upload(
    State(state): State<AppState>,
    Json(req): Json<InitUploadRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let upload_id = Uuid::new_v4().to_string();
    let temp_path = state.config.storage.base_path
        .join(".temp")
        .join(&upload_id);

    // Create temp directory
    tokio::fs::create_dir_all(&temp_path)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Record in database
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
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    Ok(Json(InitUploadResponse { upload_id }))
}

pub async fn upload_chunk(
    State(state): State<AppState>,
    Query(query): Query<UploadChunkQuery>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, StatusCode> {
    // Get upload session
    let session = {
        let db = state.db.lock().await;
        db.get_upload_session(&query.upload_id)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    };

    let session = session.ok_or(StatusCode::NOT_FOUND)?;

    // Receive chunk data
    let mut chunk_data: Option<Vec<u8>> = None;
    while let Some(field) = multipart.next_field().await.map_err(|_| StatusCode::BAD_REQUEST)? {
        if field.name() == Some("chunk") {
            let data = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;
            chunk_data = Some(data.to_vec());
        }
    }

    let chunk_data = chunk_data.ok_or(StatusCode::BAD_REQUEST)?;

    // Save chunk to temp file
    let chunk_path = std::path::Path::new(&session.temp_path)
        .join(format!("chunk_{}", query.chunk_index));

    let mut file = tokio::fs::File::create(&chunk_path)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    file.write_all(&chunk_data)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Update progress
    let received_chunks = query.chunk_index + 1;
    let completed = received_chunks >= session.total_chunks;

    {
        let db = state.db.lock().await;
        let received_bytes = (received_chunks as i64) * (chunk_data.len() as i64);
        db.update_upload_progress(&query.upload_id, query.chunk_index, received_bytes, completed)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    Ok(Json(UploadStatus {
        upload_id: query.upload_id,
        received_chunks,
        total_chunks: session.total_chunks,
        complete: completed,
    }))
}

pub async fn complete_upload(
    State(state): State<AppState>,
    Path(upload_id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    // Get upload session
    let session = {
        let db = state.db.lock().await;
        db.get_upload_session(&upload_id)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    };

    let session = session.ok_or(StatusCode::NOT_FOUND)?;

    // Combine chunks
    let album_path = state.config.storage.base_path.join(&session.album);
    tokio::fs::create_dir_all(&album_path)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let final_path = album_path.join(&session.filename);
    let mut final_file = tokio::fs::File::create(&final_path)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    for i in 0..session.total_chunks {
        let chunk_path = std::path::Path::new(&session.temp_path).join(format!("chunk_{}", i));
        let chunk_data = tokio::fs::read(&chunk_path)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        final_file
            .write_all(&chunk_data)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        // Delete chunk
        let _ = tokio::fs::remove_file(&chunk_path).await;
    }

    // Clean up temp directory
    let _ = tokio::fs::remove_dir_all(&session.temp_path).await;

    // Calculate hash and save to database
    let file_data = tokio::fs::read(&final_path)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let file_hash = format!("{:x}", sha2::Sha256::digest(&file_data));

    // Try HEIC conversion
    let converter = crate::converter::HeicConverter::new(state.config.heic_converter.clone());
    let has_jpeg = if let Ok(Some(_jpeg_path)) = converter.convert(&final_path) {
        println!("Converted HEIC to JPEG: {:?}", _jpeg_path);
        true
    } else {
        false
    };

    // Save to database
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
        };
        db.insert_photo(&photo)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    // Show notification
    crate::notify::show_upload_complete(1, &session.album);

    // Trigger cloud sync if enabled
    let sync_manager = crate::sync::SyncManager::new(state.config.clone());
    if state.config.sync.auto_sync {
        let _ = sync_manager.spawn_sync_task(state.config.sync.sync_delay_seconds);
    }

    // Clean up upload session
    {
        let db = state.db.lock().await;
        db.delete_upload_session(&upload_id).ok();
    }

    Ok(Json(serde_json::json!({
        "success": true,
        "filename": session.filename,
        "album": session.album,
        "size": session.total_size
    })))
}

pub async fn get_upload_status(
    State(state): State<AppState>,
    Path(upload_id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let session = {
        let db = state.db.lock().await;
        db.get_upload_session(&upload_id)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    };

    if let Some(session) = session {
        Ok(Json(UploadStatus {
            upload_id,
            received_chunks: session.chunk_index,
            total_chunks: session.total_chunks,
            complete: session.completed,
        }))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}
