use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Serialize;
use tracing::{info, warn};

use crate::models::TaskStatus;
use crate::server::AppState;

#[derive(Debug, Serialize)]
pub struct UploadStatusResponse {
    pub id: String,
    pub filename: String,
    pub album: String,
    pub progress_percent: i32,
    pub received_bytes: i64,
    pub total_bytes: i64,
    pub status: String,
}

/// GET /api/uploads/active - Get active uploads
pub async fn list_active_uploads(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    let db = state.db.lock().await;
    let tasks = db
        .list_active_uploads()
        .map_err(|e| {
            tracing::error!("Failed to list active uploads: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let responses: Vec<UploadStatusResponse> = tasks
        .into_iter()
        .map(|task| {
            let progress = if task.total_bytes > 0 {
                ((task.received_bytes as f64 / task.total_bytes as f64) * 100.0) as i32
            } else {
                0
            };

            UploadStatusResponse {
                id: task.id.clone(),
                filename: task.filename,
                album: task.album,
                progress_percent: progress.min(100),
                received_bytes: task.received_bytes,
                total_bytes: task.total_bytes,
                status: task.status.as_str().to_string(),
            }
        })
        .collect();

    Ok(Json(responses))
}

/// POST /api/uploads/:id/cancel - Cancel a single upload
pub async fn cancel_upload(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    info!(upload_id = %id, "Cancelling upload");

    // Check for active cancellation token
    let mut active_uploads = state.active_uploads.lock().await;
    if let Some(token) = active_uploads.remove(&id) {
        token.cancel();

        // Update database status
        let db = state.db.lock().await;
        if let Ok(Some(mut task)) = db.get_upload_task(&id) {
            task.status = TaskStatus::Cancelled;
            task.updated_at = chrono::Utc::now();
            let _ = db.create_upload_task(&task);
        }

        info!(upload_id = %id, "Upload cancelled successfully");
        return Ok(Json(
            serde_json::json!({"success": true, "message": "Upload cancelled"}),
        ));
    }

    warn!(upload_id = %id, "Upload not found for cancellation");
    Err(StatusCode::NOT_FOUND)
}

/// POST /api/uploads/cancel-all - Cancel all active uploads
pub async fn cancel_all_uploads(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    let mut active_uploads = state.active_uploads.lock().await;
    let count = active_uploads.len();

    info!(count = count, "Cancelling all active uploads");

    for (id, token) in active_uploads.drain() {
        token.cancel();

        let db = state.db.lock().await;
        if let Ok(Some(mut task)) = db.get_upload_task(&id) {
            task.status = TaskStatus::Cancelled;
            task.updated_at = chrono::Utc::now();
            let _ = db.create_upload_task(&task);
        }
    }

    info!(cancelled_count = count, "All uploads cancelled");
    Ok(Json(serde_json::json!({
        "success": true,
        "cancelled_count": count
    })))
}

/// DELETE /api/uploads/cleanup-incomplete - Clean up incomplete uploads (admin feature)
pub async fn cleanup_incomplete_uploads(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    info!("Cleaning up incomplete uploads");

    // Clean up database records
    let db = state.db.lock().await;
    let cleaned_count = db.cleanup_incomplete_uploads().map_err(|e| {
        tracing::error!("Failed to cleanup incomplete uploads: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Clean up temp files
    let temp_dir = state.config.storage.base_path.join(".temp");
    let mut freed_bytes: i64 = 0;

    if let Ok(entries) = std::fs::read_dir(&temp_dir) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata()
                && metadata.is_dir()
            {
                freed_bytes += dir_size(&entry.path()) as i64;
                let _ = std::fs::remove_dir_all(entry.path());
            }
        }
    }

    info!(
        cleaned_count = cleaned_count,
        freed_bytes = freed_bytes,
        "Cleanup completed"
    );

    Ok(Json(serde_json::json!({
        "success": true,
        "cleaned_count": cleaned_count,
        "freed_bytes": freed_bytes
    })))
}

/// Calculate directory size recursively
fn dir_size(path: &std::path::Path) -> u64 {
    let mut size = 0;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    size += metadata.len();
                } else if metadata.is_dir() {
                    size += dir_size(&entry.path());
                }
            }
        }
    }
    size
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_upload_status_response_serialization() {
        let response = UploadStatusResponse {
            id: "test-123".to_string(),
            filename: "photo.jpg".to_string(),
            album: "vacation".to_string(),
            progress_percent: 50,
            received_bytes: 512,
            total_bytes: 1024,
            status: "uploading".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("test-123"));
        assert!(json.contains("photo.jpg"));
        assert!(json.contains("50"));
    }
}
