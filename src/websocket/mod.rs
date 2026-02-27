use axum::{
    extract::State,
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
};
use tokio::sync::broadcast;

use crate::server::AppState;

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum WsEvent {
    /// Upload session initialized
    UploadStarted {
        upload_id: String,
        filename: String,
        album: String,
        total_bytes: i64,
        total_chunks: i32,
    },
    /// Individual chunk received
    ChunkReceived {
        upload_id: String,
        chunk_index: i32,
        total_chunks: i32,
    },
    /// Overall upload progress (percentage)
    UploadProgress {
        upload_id: String,
        filename: String,
        received_bytes: i64,
        total_bytes: i64,
        percent: u8,
    },
    /// All chunks received, starting to merge
    ChunksMerging {
        upload_id: String,
        filename: String,
    },
    /// File saved to disk
    FileSaved {
        upload_id: String,
        filename: String,
        path: String,
        size: i64,
    },
    /// HEIC conversion in progress
    HeicConverting {
        upload_id: String,
        filename: String,
    },
    /// HEIC conversion result
    HeicConverted {
        upload_id: String,
        original: String,
        converted: String,
        success: bool,
    },
    /// Saving to database
    DatabaseSaving {
        upload_id: String,
        filename: String,
    },
    /// Upload completed successfully
    UploadComplete {
        upload_id: String,
        filename: String,
        album: String,
        size: i64,
    },
    /// Upload failed with error
    UploadError {
        upload_id: String,
        filename: String,
        error: String,
        stage: String, // Which stage failed: "init", "chunk", "merge", "convert", "database"
    },
    /// Cloud sync triggered after upload
    CloudSyncTriggered {
        upload_id: String,
        filename: String,
    },
    /// Cloud sync started
    CloudSyncStarted,
    /// Cloud sync completed
    CloudSyncComplete { success: bool },
}

pub type EventSender = broadcast::Sender<WsEvent>;
pub type EventReceiver = broadcast::Receiver<WsEvent>;

pub fn create_event_channel() -> (EventSender, EventReceiver) {
    broadcast::channel(100)
}

pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    let mut rx = state.event_sender.subscribe();

    loop {
        tokio::select! {
            Ok(event) = rx.recv() => {
                let msg = serialize_event(event);

                if socket.send(Message::Text(msg.to_string())).await.is_err() {
                    break;
                }
            }
            else => break,
        }
    }
}

fn serialize_event(event: WsEvent) -> serde_json::Value {
    match event {
        WsEvent::UploadStarted { upload_id, filename, album, total_bytes, total_chunks } => {
            serde_json::json!({
                "type": "upload_started",
                "upload_id": upload_id,
                "filename": filename,
                "album": album,
                "total_bytes": total_bytes,
                "total_chunks": total_chunks
            })
        }
        WsEvent::ChunkReceived { upload_id, chunk_index, total_chunks } => {
            serde_json::json!({
                "type": "chunk_received",
                "upload_id": upload_id,
                "chunk_index": chunk_index,
                "total_chunks": total_chunks
            })
        }
        WsEvent::UploadProgress { upload_id, filename, received_bytes, total_bytes, percent } => {
            serde_json::json!({
                "type": "upload_progress",
                "upload_id": upload_id,
                "filename": filename,
                "received_bytes": received_bytes,
                "total_bytes": total_bytes,
                "percent": percent
            })
        }
        WsEvent::ChunksMerging { upload_id, filename } => {
            serde_json::json!({
                "type": "chunks_merging",
                "upload_id": upload_id,
                "filename": filename
            })
        }
        WsEvent::FileSaved { upload_id, filename, path, size } => {
            serde_json::json!({
                "type": "file_saved",
                "upload_id": upload_id,
                "filename": filename,
                "path": path,
                "size": size
            })
        }
        WsEvent::HeicConverting { upload_id, filename } => {
            serde_json::json!({
                "type": "heic_converting",
                "upload_id": upload_id,
                "filename": filename
            })
        }
        WsEvent::HeicConverted { upload_id, original, converted, success } => {
            serde_json::json!({
                "type": "heic_converted",
                "upload_id": upload_id,
                "original": original,
                "converted": converted,
                "success": success
            })
        }
        WsEvent::DatabaseSaving { upload_id, filename } => {
            serde_json::json!({
                "type": "database_saving",
                "upload_id": upload_id,
                "filename": filename
            })
        }
        WsEvent::UploadComplete { upload_id, filename, album, size } => {
            serde_json::json!({
                "type": "upload_complete",
                "upload_id": upload_id,
                "filename": filename,
                "album": album,
                "size": size
            })
        }
        WsEvent::UploadError { upload_id, filename, error, stage } => {
            serde_json::json!({
                "type": "upload_error",
                "upload_id": upload_id,
                "filename": filename,
                "error": error,
                "stage": stage
            })
        }
        WsEvent::CloudSyncTriggered { upload_id, filename } => {
            serde_json::json!({
                "type": "cloud_sync_triggered",
                "upload_id": upload_id,
                "filename": filename
            })
        }
        WsEvent::CloudSyncStarted => {
            serde_json::json!({"type": "cloud_sync_started"})
        }
        WsEvent::CloudSyncComplete { success } => {
            serde_json::json!({
                "type": "cloud_sync_complete",
                "success": success
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that all WsEvent types serialize correctly with expected fields
    #[test]
    fn test_event_serialization() {
        // Test UploadStarted
        let event = WsEvent::UploadStarted {
            upload_id: "test-123".to_string(),
            filename: "photo.jpg".to_string(),
            album: "vacation".to_string(),
            total_bytes: 5242880,
            total_chunks: 10,
        };
        let json = serialize_event(event);
        assert_eq!(json["type"], "upload_started");
        assert_eq!(json["upload_id"], "test-123");
        assert_eq!(json["filename"], "photo.jpg");
        assert_eq!(json["total_chunks"], 10);

        // Test ChunkReceived
        let event = WsEvent::ChunkReceived {
            upload_id: "test-123".to_string(),
            chunk_index: 5,
            total_chunks: 10,
        };
        let json = serialize_event(event);
        assert_eq!(json["type"], "chunk_received");
        assert_eq!(json["chunk_index"], 5);
        assert_eq!(json["total_chunks"], 10);

        // Test UploadProgress
        let event = WsEvent::UploadProgress {
            upload_id: "test-123".to_string(),
            filename: "photo.jpg".to_string(),
            received_bytes: 2621440,
            total_bytes: 5242880,
            percent: 50,
        };
        let json = serialize_event(event);
        assert_eq!(json["type"], "upload_progress");
        assert_eq!(json["percent"], 50);
        assert_eq!(json["received_bytes"], 2621440);
        assert_eq!(json["total_bytes"], 5242880);

        // Test UploadComplete
        let event = WsEvent::UploadComplete {
            upload_id: "test-123".to_string(),
            filename: "photo.jpg".to_string(),
            album: "vacation".to_string(),
            size: 5242880,
        };
        let json = serialize_event(event);
        assert_eq!(json["type"], "upload_complete");
        assert_eq!(json["size"], 5242880);

        // Test UploadError
        let event = WsEvent::UploadError {
            upload_id: "test-123".to_string(),
            filename: "photo.jpg".to_string(),
            error: "Disk full".to_string(),
            stage: "merge".to_string(),
        };
        let json = serialize_event(event);
        assert_eq!(json["type"], "upload_error");
        assert_eq!(json["error"], "Disk full");
        assert_eq!(json["stage"], "merge");

        // Test CloudSync events
        let event = WsEvent::CloudSyncStarted;
        let json = serialize_event(event);
        assert_eq!(json["type"], "cloud_sync_started");

        let event = WsEvent::CloudSyncComplete { success: true };
        let json = serialize_event(event);
        assert_eq!(json["type"], "cloud_sync_complete");
        assert_eq!(json["success"], true);
    }

    /// Test event channel creation and basic send/receive
    #[test]
    fn test_event_channel() {
        let (sender, mut receiver) = create_event_channel();

        // Send an event
        let event = WsEvent::UploadStarted {
            upload_id: "test-123".to_string(),
            filename: "photo.jpg".to_string(),
            album: "vacation".to_string(),
            total_bytes: 1000,
            total_chunks: 5,
        };
        sender.send(event.clone()).unwrap();

        // Receive the event
        let received = receiver.try_recv().unwrap();
        match received {
            WsEvent::UploadStarted { upload_id, filename, .. } => {
                assert_eq!(upload_id, "test-123");
                assert_eq!(filename, "photo.jpg");
            }
            _ => panic!("Expected UploadStarted event"),
        }
    }

    /// Test event cloning (needed for broadcast)
    #[test]
    fn test_event_clone() {
        let event = WsEvent::UploadProgress {
            upload_id: "test-123".to_string(),
            filename: "photo.jpg".to_string(),
            received_bytes: 500,
            total_bytes: 1000,
            percent: 50,
        };
        let cloned = event.clone();

        match cloned {
            WsEvent::UploadProgress { percent, .. } => {
                assert_eq!(percent, 50);
            }
            _ => panic!("Expected UploadProgress event"),
        }
    }
}
