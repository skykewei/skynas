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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
pub struct SyncHistory {
    pub id: i64,
    pub photo_id: i64,
    pub operation_id: i64,
    pub status: SyncStatus,
    pub synced_at: Option<DateTime<Utc>>,
    pub error_msg: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
