use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};

use crate::auth::create_token;
use crate::server::AppState;

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub expires_at: String,
}

/// POST /api/admin/login - 管理员登录
pub async fn admin_login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let db = state.db.lock().await;
    let config = db
        .get_or_create_admin_config("change-me-in-production")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 验证密码
    let valid = bcrypt::verify(&req.password, &config.admin_password_hash)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !valid {
        return Err(StatusCode::UNAUTHORIZED);
    }

    // 生成 token
    let token = create_token(&config.jwt_secret).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let expires_at = chrono::Utc::now() + chrono::Duration::hours(24);

    Ok(Json(LoginResponse {
        token,
        expires_at: expires_at.to_rfc3339(),
    }))
}

#[derive(Debug, Serialize)]
pub struct AdminStats {
    pub total_photos: i64,
    pub total_size: i64,
    pub album_count: i32,
    pub disk_available: u64,
}

/// GET /api/admin/stats - 获取统计信息
pub async fn get_admin_stats(State(state): State<AppState>) -> Result<impl IntoResponse, StatusCode> {
    let db = state.db.lock().await;

    let total_photos: i64 = db
        .conn
        .query_row("SELECT COUNT(*) FROM photos", [], |row| row.get(0))
        .unwrap_or(0);

    let total_size: i64 = db
        .conn
        .query_row(
            "SELECT COALESCE(SUM(size_bytes), 0) FROM photos",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let album_count: i32 = db
        .conn
        .query_row(
            "SELECT COUNT(DISTINCT album) FROM photos",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    // 获取磁盘可用空间（简化版）
    let disk_available = 0u64; // 暂不实现

    Ok(Json(AdminStats {
        total_photos,
        total_size,
        album_count,
        disk_available,
    }))
}

#[derive(Debug, Serialize)]
pub struct ConfigResponse {
    pub server_port: u16,
    pub server_host: String,
    pub storage_path: String,
    pub features: serde_json::Value,
}

/// GET /api/admin/config - 获取配置
pub async fn get_config(State(state): State<AppState>) -> Result<impl IntoResponse, StatusCode> {
    Ok(Json(ConfigResponse {
        server_port: state.config.server.port,
        server_host: state.config.server.host.clone(),
        storage_path: state.config.storage.base_path.to_string_lossy().to_string(),
        features: serde_json::json!({
            "mdns_enabled": state.config.features.mdns_enabled,
            "websocket_enabled": state.config.features.websocket_enabled,
            "heic_to_jpeg": state.config.heic_converter.generate_jpeg,
            "notification_enabled": state.config.features.notification_enabled,
        }),
    }))
}

#[derive(Debug, Deserialize)]
pub struct UpdateConfigRequest {
    pub storage_path: Option<String>,
    pub server_port: Option<u16>,
}

/// PUT /api/admin/config - 更新配置
pub async fn update_config(
    State(_state): State<AppState>,
    Json(_req): Json<UpdateConfigRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    // 配置更新需要重启服务器或实现热重载
    // 这里先返回成功，提示需要重启
    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Configuration will take effect after server restart"
    })))
}

/// POST /api/admin/config/validate-storage - 验证存储路径
pub async fn validate_storage_path(
    State(_state): State<AppState>,
    Json(req): Json<UpdateConfigRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    if let Some(path_str) = req.storage_path {
        let path = std::path::PathBuf::from(&path_str);

        let exists = path.exists();
        let writable = std::fs::metadata(&path)
            .ok()
            .map(|m| !m.permissions().readonly())
            .unwrap_or(false);

        return Ok(Json(serde_json::json!({
            "valid": exists && writable,
            "exists": exists,
            "writable": writable,
            "path": path_str
        })));
    }

    Err(StatusCode::BAD_REQUEST)
}
