use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{
        ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::Response,
};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use uuid::Uuid;

use auth_service_client::AuthClient;
use common_errors::AppError;

use super::AppState;

// Kode close WebSocket — kontrak protokol (dipakai/disiapkan untuk penolakan koneksi).
#[allow(dead_code)]
const CLOSE_UNAUTHORIZED: u16 = 4001;
#[allow(dead_code)]
const CLOSE_FORBIDDEN: u16 = 4003;
#[allow(dead_code)]
const CLOSE_NOT_FOUND: u16 = 4004;
const CLOSE_GOING_AWAY: u16 = 1012;
const KEEPALIVE_SECS: u64 = 60;

#[derive(Deserialize)]
pub struct WsQuery {
    token: String,
    conversation_id: Uuid,
}

/// Envelope untuk semua pesan WebSocket.
#[derive(Clone, Serialize, Deserialize)]
pub struct WsEnvelope {
    pub r#type: String,
    pub payload: serde_json::Value,
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(q): Query<WsQuery>,
    State(s): State<AppState>,
) -> Result<Response, AppError> {
    // Auth sebelum upgrade — close 4001 jika token invalid
    let claims = s
        .auth_client
        .validate_token(&q.token)
        .await
        .map_err(|_| AppError::Unauthorized)?;

    let user_id = claims.user_id;
    let conv_id = q.conversation_id;

    // Subscribe ke broadcast channel untuk conversation ini.
    // Channel dibuat otomatis jika belum ada; di-cleanup saat 0 subscriber.
    let rx = {
        let mut rooms = s.conversation_rooms.lock().await;
        let entry = rooms
            .entry(conv_id)
            .or_insert_with(|| broadcast::channel(64).0);
        entry.subscribe()
    };

    Ok(ws.on_upgrade(move |socket| {
        handle_socket(
            socket,
            user_id,
            conv_id,
            rx,
            Arc::clone(&s.auth_client),
            s.conversation_rooms.clone(),
        )
    }))
}

use super::ConversationRooms;

async fn handle_socket(
    mut socket: WebSocket,
    user_id: Uuid,
    conv_id: Uuid,
    mut broadcast_rx: broadcast::Receiver<WsEnvelope>,
    _auth: Arc<dyn AuthClient>,
    conversation_rooms: ConversationRooms,
) {
    let keepalive = Duration::from_secs(KEEPALIVE_SECS);

    let result: Result<(), ()> = loop {
        // Use tokio::select to handle broadcast and recv simultaneously
        tokio::select! {
            // Broadcast: relay message dari participant lain ke client ini
            broadcast_msg = broadcast_rx.recv() => {
                match broadcast_msg {
                    Ok(envelope) => {
                        let payload = serde_json::to_string(&envelope).unwrap_or_default();
                        if socket.send(Message::Text(payload.into())).await.is_err() {
                            break Err(());
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!(user_id = %user_id, conv_id = %conv_id, lagged = n,
                            "ws client lagged — beberapa pesan hilang");
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        break Err(());
                    }
                }
            }

            // Receive: baca pesan dari client ini
            recv = tokio::time::timeout(keepalive, socket.recv()) => {
                match recv {
                    // Timeout → send ping
                    Err(_) => {
                        if socket.send(Message::Ping(vec![].into())).await.is_err() {
                            tracing::info!(user_id = %user_id, "ws client disconnected (ping failed)");
                            break Err(());
                        }
                    }

                    // Connection closed cleanly
                    Ok(None) => {
                        tracing::info!(user_id = %user_id, conv_id = %conv_id, "ws client disconnected");
                        break Ok(());
                    }

                    Ok(Some(Err(e))) => {
                        tracing::warn!(user_id = %user_id, error = ?e, "ws error");
                        break Err(());
                    }

                    Ok(Some(Ok(msg))) => {
                        match msg {
                            Message::Text(text) => {
                                match serde_json::from_str::<WsEnvelope>(&text) {
                                    Ok(envelope) => {
                                        tracing::debug!(
                                            user_id  = %user_id,
                                            conv_id  = %conv_id,
                                            r#type   = %envelope.r#type,
                                            "ws message received"
                                        );
                                        let ack = WsEnvelope {
                                            r#type: "ack".into(),
                                            payload: serde_json::json!({ "type": envelope.r#type }),
                                        };
                                        let _ = socket
                                            .send(Message::Text(
                                                serde_json::to_string(&ack).unwrap_or_default().into(),
                                            ))
                                            .await;
                                    }
                                    Err(e) => {
                                        tracing::warn!(user_id = %user_id, error = ?e, "invalid ws envelope");
                                    }
                                }
                            }

                            Message::Close(_) => {
                                let _ = socket
                                    .send(Message::Close(Some(CloseFrame {
                                        code: CLOSE_GOING_AWAY,
                                        reason: "server shutting down".into(),
                                    })))
                                    .await;
                                break Ok(());
                            }

                            Message::Pong(_) => {}

                            _ => {}
                        }
                    }
                }
            }
        }
    };

    // ── Cleanup: hapus entry dari HashMap jika tidak ada subscriber lagi ──
    let mut rooms = conversation_rooms.lock().await;
    if let Some(tx) = rooms.get(&conv_id) {
        if tx.receiver_count() == 0 {
            rooms.remove(&conv_id);
            tracing::debug!(conv_id = %conv_id, "rooms: removed empty channel");
        }
    }
    drop(rooms);

    let _ = result; // suppress unused warning
}
