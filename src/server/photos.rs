use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::server::AppState;

#[derive(Debug, Deserialize)]
pub struct ListPhotosQuery {
    pub page: Option<i32>,
    pub limit: Option<i32>,
    pub album: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ListPhotosResponse {
    pub photos: Vec<PhotoItem>,
    pub total: i64,
    pub page: i32,
    pub limit: i32,
}

#[derive(Debug, Serialize)]
pub struct PhotoItem {
    pub id: i64,
    pub filename: String,
    pub album: String,
    pub size_bytes: i64,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub uploaded_at: String,
    pub thumbnail_url: Option<String>,
}

impl From<crate::models::Photo> for PhotoItem {
    fn from(photo: crate::models::Photo) -> Self {
        let thumbnail_url = photo.thumbnail_path.as_ref().map(|_| format!("/api/photos/{}/thumbnail", photo.id));
        Self {
            id: photo.id,
            filename: photo.filename,
            album: photo.album,
            size_bytes: photo.size_bytes,
            width: photo.width,
            height: photo.height,
            uploaded_at: photo.uploaded_at.to_rfc3339(),
            thumbnail_url,
        }
    }
}

/// GET /api/photos - 获取照片列表（分页）
pub async fn list_photos(
    State(state): State<AppState>,
    Query(query): Query<ListPhotosQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) as i64 * limit as i64;

    let db = state.db.lock().await;

    let album_ref = query.album.as_deref();
    let (photos, total) = db.list_photos(album_ref, limit, offset)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let photo_items: Vec<PhotoItem> = photos.into_iter().map(PhotoItem::from).collect();

    Ok(Json(ListPhotosResponse {
        photos: photo_items,
        total,
        page,
        limit,
    }))
}

/// GET /api/photos/:id - 获取单张照片详情
pub async fn get_photo(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, StatusCode> {
    let db = state.db.lock().await;

    let photo = db.get_photo(id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match photo {
        Some(p) => Ok(Json(PhotoItem::from(p))),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// GET /api/photos/:id/thumbnail - 获取缩略图
pub async fn get_thumbnail(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, StatusCode> {
    use axum::response::Response;
    use axum::body::Body;
    use axum::http::header;

    let db = state.db.lock().await;

    let thumbnail_path = db.get_thumbnail_path(id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some(path) = thumbnail_path {
        if let Ok(bytes) = tokio::fs::read(&path).await {
            return Ok(Response::builder()
                .header(header::CONTENT_TYPE, "image/jpeg")
                .body(Body::from(bytes))
                .unwrap());
        }
    }

    Err(StatusCode::NOT_FOUND)
}

/// GET /api/albums - 获取相册列表
pub async fn list_albums(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    let db = state.db.lock().await;

    let albums = db.list_albums()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let result: Vec<HashMap<String, String>> = albums
        .into_iter()
        .map(|(name, count)| {
            HashMap::from([
                ("name".to_string(), name),
                ("count".to_string(), count.to_string()),
            ])
        })
        .collect();

    Ok(Json(result))
}

/// GET /api/photos/:id/image - 获取原图
pub async fn get_image(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, StatusCode> {
    use axum::response::Response;
    use axum::body::Body;
    use axum::http::header;

    let db = state.db.lock().await;

    // 查询照片路径
    let (local_path, has_jpeg): (String, bool) = db.conn.query_row(
        "SELECT local_path, has_jpeg_variant FROM photos WHERE id = ?1",
        [id],
        |row| Ok((row.get(0)?, row.get(1)?))
    ).map_err(|_| StatusCode::NOT_FOUND)?;

    // 如果存在 JPEG 变体，返回 JPEG
    let file_path = if has_jpeg {
        let jpeg_path = std::path::PathBuf::from(&local_path)
            .with_extension("jpg");
        if jpeg_path.exists() {
            jpeg_path
        } else {
            std::path::PathBuf::from(&local_path)
        }
    } else {
        std::path::PathBuf::from(&local_path)
    };

    // 读取文件
    match tokio::fs::read(&file_path).await {
        Ok(bytes) => {
            // 根据扩展名设置 Content-Type
            let content_type = match file_path.extension().and_then(|e| e.to_str()) {
                Some("jpg") | Some("jpeg") => "image/jpeg",
                Some("png") => "image/png",
                Some("webp") => "image/webp",
                Some("heic") => "image/heic",
                Some("mp4") | Some("mov") => "video/mp4",
                _ => "application/octet-stream",
            };

            Ok(Response::builder()
                .header(header::CONTENT_TYPE, content_type)
                .header(header::CACHE_CONTROL, "public, max-age=31536000")
                .body(Body::from(bytes))
                .unwrap())
        }
        Err(e) => {
            tracing::error!("Failed to read image file {}: {}", file_path.display(), e);
            Err(StatusCode::NOT_FOUND)
        }
    }
}

/// DELETE /api/photos/:id - 删除照片
pub async fn delete_photo(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, StatusCode> {
    let db = state.db.lock().await;

    // 获取照片信息（路径和缩略图路径）
    let (local_path, thumbnail_path): (String, Option<String>) = db.conn.query_row(
        "SELECT local_path, thumbnail_path FROM photos WHERE id = ?1",
        [id],
        |row| Ok((row.get(0)?, row.get(1)?))
    ).map_err(|_| StatusCode::NOT_FOUND)?;

    // 删除数据库记录
    db.conn.execute(
        "DELETE FROM photos WHERE id = ?1",
        [id],
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 删除主文件
    let main_path = std::path::PathBuf::from(&local_path);
    if let Err(e) = tokio::fs::remove_file(&main_path).await {
        tracing::warn!("Failed to delete main file {}: {}", main_path.display(), e);
    }

    // 删除 JPEG 变体（如果存在）
    let jpeg_path = main_path.with_extension("jpg");
    if jpeg_path.exists() {
        if let Err(e) = tokio::fs::remove_file(&jpeg_path).await {
            tracing::warn!("Failed to delete JPEG variant {}: {}", jpeg_path.display(), e);
        }
    }

    // 删除缩略图
    if let Some(thumb) = thumbnail_path {
        let thumb_path = std::path::PathBuf::from(&thumb);
        if let Err(e) = tokio::fs::remove_file(&thumb_path).await {
            tracing::warn!("Failed to delete thumbnail {}: {}", thumb_path.display(), e);
        }
    }

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Photo deleted successfully"
    })))
}
