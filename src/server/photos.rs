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
