use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::State,
    response::IntoResponse,
};
use tokio::sync::broadcast;

use crate::server::AppState;

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum WsEvent {
    UploadStarted { filename: String, album: String },
    UploadProgress { filename: String, percent: u8 },
    UploadComplete { filename: String, album: String },
    UploadError { filename: String, error: String },
    CloudSyncStarted,
    CloudSyncComplete { success: bool },
}

pub type EventSender = broadcast::Sender<WsEvent>;
pub type EventReceiver = broadcast::Receiver<WsEvent>;

pub fn create_event_channel() -> (EventSender, EventReceiver) {
    broadcast::channel(100)
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    let mut rx = state.event_sender.subscribe();

    loop {
        tokio::select! {
            Ok(event) = rx.recv() => {
                let msg = match event {
                    WsEvent::UploadStarted { filename, album } => {
                        serde_json::json!({
                            "type": "upload_started",
                            "filename": filename,
                            "album": album
                        })
                    }
                    WsEvent::UploadProgress { filename, percent } => {
                        serde_json::json!({
                            "type": "upload_progress",
                            "filename": filename,
                            "percent": percent
                        })
                    }
                    WsEvent::UploadComplete { filename, album } => {
                        serde_json::json!({
                            "type": "upload_complete",
                            "filename": filename,
                            "album": album
                        })
                    }
                    WsEvent::UploadError { filename, error } => {
                        serde_json::json!({
                            "type": "upload_error",
                            "filename": filename,
                            "error": error
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
                };

                if socket.send(Message::Text(msg.to_string())).await.is_err() {
                    break;
                }
            }
            else => break,
        }
    }
}
