use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Photo {
    pub id: i64,
    pub filename: String,
    pub album: String,
    pub file_hash: Option<String>,
    pub size_bytes: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub uploaded_at: DateTime<Utc>,
    pub local_path: String,
    pub has_jpeg_variant: bool,
    pub thumbnail_path: Option<String>,  // 缩略图路径
    pub width: Option<i32>,              // 图片宽度
    pub height: Option<i32>,             // 图片高度
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct SyncOperation {
    pub id: i64,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub total_files: i32,
    pub success_count: i32,
    pub fail_count: i32,
    pub client_ip: Option<String>,
    pub user_agent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct SyncHistory {
    pub id: i64,
    pub photo_id: i64,
    pub operation_id: i64,
    pub status: SyncStatus,
    pub synced_at: Option<DateTime<Utc>>,
    pub error_msg: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub enum SyncStatus {
    Pending,
    Synced,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadChunk {
    pub upload_id: String,
    pub filename: String,
    pub album: String,
    pub total_size: i64,
    pub chunk_index: i32,
    pub total_chunks: i32,
    pub received_bytes: i64,
    pub completed: bool,
    pub created_at: DateTime<Utc>,
    pub temp_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadTask {
    pub id: String,
    pub filename: String,
    pub album: String,
    pub total_bytes: i64,
    pub received_bytes: i64,
    pub status: TaskStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub cancelled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    Uploading,
    Completed,
    Cancelled,
    Error,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "pending",
            TaskStatus::Uploading => "uploading",
            TaskStatus::Completed => "completed",
            TaskStatus::Cancelled => "cancelled",
            TaskStatus::Error => "error",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminConfig {
    pub id: i64,
    pub jwt_secret: String,
    pub admin_password_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
