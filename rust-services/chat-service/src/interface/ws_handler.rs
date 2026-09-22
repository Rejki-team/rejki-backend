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

use common_errors::AppError;

use super::AppState;
use crate::application::dto::SendMessageInput;
use crate::application::service::ChatService;
use crate::infrastructure::pg_repository::PgChatRepository;

// Kode close WebSocket — kontrak protokol (docs/websocket-contract.html). 4003/4004
// TIDAK dipakai di alur handshake saat ini (celah keanggotaan ditolak via HTTP 403/404
// SEBELUM upgrade — sesuai kontrak "Room tidak ada/akses ditolak → Tolak dengan HTTP
// 404/403 saat handshake", bukan close frame pasca-upgrade). Disiapkan untuk skenario
// masa depan (mis. membership berubah di tengah koneksi).
#[allow(dead_code)]
const CLOSE_UNAUTHORIZED: u16 = 4001;
#[allow(dead_code)]
const CLOSE_FORBIDDEN: u16 = 4003;
#[allow(dead_code)]
const CLOSE_NOT_FOUND: u16 = 4004;
const CLOSE_GOING_AWAY: u16 = 1012;
const KEEPALIVE_SECS: u64 = 60;
/// Batas koneksi WS aktif per user (Audit Finding #3, 2026-09-22) — mencegah satu user
/// membuka koneksi tak terbatas (resource exhaustion di proses yang sama dengan service lain).
const MAX_WS_CONNECTIONS_PER_USER: u32 = 5;

#[derive(Deserialize)]
pub struct WsQuery {
    token: String,
    conversation_id: Uuid,
}

/// Envelope untuk semua pesan WebSocket — sesuai `docs/websocket-contract.html`
/// ({ type, request_id, payload }). `request_id` opsional saat deserialize (client
/// lama/rusak tidak boleh membuat server panic), tapi WAJIB diisi client per kontrak.
#[derive(Clone, Serialize, Deserialize)]
pub struct WsEnvelope {
    pub r#type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    pub payload: serde_json::Value,
}

impl WsEnvelope {
    fn new(r#type: &str, request_id: Option<String>, payload: serde_json::Value) -> Self {
        Self {
            r#type: r#type.to_string(),
            request_id,
            payload,
        }
    }
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

    // P4.0: membership check SEBELUM upgrade — kontrak: "Room tidak ada / akses
    // ditolak → Tolak koneksi dengan HTTP 404 atau 403 saat handshake". Ownership
    // check di query (`is_participant`), bukan SELECT lalu compare (IDOR→404, §4.4
    // backend) — tapi di sini kita bisa bedakan "tidak ada" vs "bukan participant"
    // karena keduanya sama-sama harus ditolak sebelum upgrade tanpa membocorkan
    // partisipan lain; pilih 404 seragam supaya tidak membocorkan keberadaan room.
    if !s
        .chat_svc
        .is_participant(conv_id, user_id)
        .await
        .map_err(AppError::Internal)?
    {
        return Err(AppError::NotFound("conversation tidak ditemukan".into()));
    }

    // P4.8: batas koneksi WS aktif per user — tolak SEBELUM upgrade (ambang terlampaui →
    // 429, bukan close-code pasca-upgrade, konsisten dengan pola membership check P4.0).
    let current = s
        .ws_connection_counts
        .get(&user_id)
        .map(|c| *c)
        .unwrap_or(0);
    if current >= MAX_WS_CONNECTIONS_PER_USER {
        return Err(AppError::RateLimited(
            "terlalu banyak koneksi WebSocket aktif".into(),
        ));
    }
    *s.ws_connection_counts.entry(user_id).or_insert(0) += 1;

    // Subscribe ke broadcast channel untuk conversation ini.
    // Channel dibuat otomatis jika belum ada; di-cleanup saat 0 subscriber.
    let rx = {
        let entry = s
            .conversation_rooms
            .entry(conv_id)
            .or_insert_with(|| broadcast::channel(64).0);
        entry.subscribe()
    };

    let deps = SocketDeps {
        conversation_rooms: s.conversation_rooms.clone(),
        chat_svc: s.chat_svc.clone(),
        ws_connection_counts: s.ws_connection_counts.clone(),
    };
    Ok(ws.on_upgrade(move |socket| handle_socket(socket, user_id, conv_id, rx, deps)))
}

use super::{ConversationRooms, WsConnectionCounts};

/// Dependensi bersama satu koneksi WS — dibungkus struct (Zero Too Many Arguments,
/// CLAUDE.md §4.7) sejak `ws_connection_counts` ditambahkan (P4.8).
struct SocketDeps {
    conversation_rooms: ConversationRooms,
    chat_svc: Arc<ChatService<PgChatRepository>>,
    ws_connection_counts: WsConnectionCounts,
}

async fn handle_socket(
    mut socket: WebSocket,
    user_id: Uuid,
    conv_id: Uuid,
    mut broadcast_rx: broadcast::Receiver<WsEnvelope>,
    deps: SocketDeps,
) {
    let SocketDeps {
        conversation_rooms,
        chat_svc,
        ws_connection_counts,
    } = deps;
    let keepalive = Duration::from_secs(KEEPALIVE_SECS);

    // system.connected — konfirmasi koneksi berhasil (kontrak: dikirim langsung
    // setelah handshake).
    let connected = WsEnvelope::new(
        "system.connected",
        None,
        serde_json::json!({ "user_id": user_id, "conversation_id": conv_id }),
    );
    let _ = socket
        .send(Message::Text(
            serde_json::to_string(&connected).unwrap_or_default().into(),
        ))
        .await;

    let result: Result<(), ()> = loop {
        tokio::select! {
            // Broadcast: relay message dari participant lain (termasuk echo diri
            // sendiri, tapi client-side biasanya sudah optimistic-render sendiri)
            // ke client ini.
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
                                handle_text_message(
                                    &mut socket,
                                    &text,
                                    user_id,
                                    conv_id,
                                    &chat_svc,
                                    &conversation_rooms,
                                )
                                .await;
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

    // ── Cleanup: hapus entry dari registry jika tidak ada subscriber lagi ──
    // `remove_if` atomik (DashMap) — tidak perlu lock manual + get lalu remove terpisah.
    let removed = conversation_rooms.remove_if(&conv_id, |_, tx| tx.receiver_count() == 0);
    if removed.is_some() {
        tracing::debug!(conv_id = %conv_id, "rooms: removed empty channel");
    }

    // P4.8: lepas slot koneksi WS user ini.
    if let Some(mut count) = ws_connection_counts.get_mut(&user_id) {
        *count = count.saturating_sub(1);
        let now_zero = *count == 0;
        drop(count);
        if now_zero {
            ws_connection_counts.remove(&user_id);
        }
    }

    let _ = result; // suppress unused warning
}

/// P4.0/P4.2: proses satu envelope client→server (`chat.send`, `system.ping`).
/// `chat.typing`/`chat.read` TIDAK diimplementasikan fase ini (di luar scope P4.2 —
/// lokasi/foto tidak butuh indikator mengetik/dibaca; dicatat sebagai gap di plan).
async fn handle_text_message(
    socket: &mut WebSocket,
    text: &str,
    user_id: Uuid,
    conv_id: Uuid,
    chat_svc: &Arc<ChatService<PgChatRepository>>,
    conversation_rooms: &ConversationRooms,
) {
    let envelope: WsEnvelope = match serde_json::from_str(text) {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!(user_id = %user_id, error = ?e, "invalid ws envelope");
            send_error(
                socket,
                None,
                "INVALID_PAYLOAD",
                "Format envelope tidak valid",
            )
            .await;
            return;
        }
    };

    tracing::debug!(
        user_id = %user_id,
        conv_id = %conv_id,
        r#type = %envelope.r#type,
        "ws message received"
    );

    match envelope.r#type.as_str() {
        "system.ping" => {
            let pong = WsEnvelope::new("system.pong", envelope.request_id, serde_json::Value::Null);
            let _ = socket
                .send(Message::Text(
                    serde_json::to_string(&pong).unwrap_or_default().into(),
                ))
                .await;
        }
        "chat.send" => {
            let input: SendMessageInput = match serde_json::from_value(envelope.payload.clone()) {
                Ok(i) => i,
                Err(_) => {
                    send_error(
                        socket,
                        envelope.request_id.clone(),
                        "INVALID_PAYLOAD",
                        "Format payload tidak sesuai",
                    )
                    .await;
                    return;
                }
            };

            match chat_svc.send_message(conv_id, user_id, input).await {
                Ok(msg) => {
                    // Ack ke pengirim
                    let ack = WsEnvelope::new(
                        "chat.message_ack",
                        envelope.request_id.clone(),
                        serde_json::json!({ "message_id": msg.id }),
                    );
                    let _ = socket
                        .send(Message::Text(
                            serde_json::to_string(&ack).unwrap_or_default().into(),
                        ))
                        .await;

                    // Broadcast ke seluruh member room (termasuk pengirim — client
                    // idempoten terhadap echo pesan sendiri via message_id).
                    let broadcast_envelope = WsEnvelope::new(
                        "chat.message",
                        None,
                        serde_json::to_value(&msg).unwrap_or_default(),
                    );
                    if let Some(tx) = conversation_rooms.get(&conv_id) {
                        let _ = tx.send(broadcast_envelope);
                    }
                }
                Err(e) => {
                    let msg = e.to_string();
                    let code = if msg.contains("terlalu banyak permintaan") {
                        "RATE_LIMITED"
                    } else if msg.contains("1-4000 karakter") {
                        "MESSAGE_TOO_LONG"
                    } else if msg.contains("tidak ditemukan") {
                        "ROOM_NOT_MEMBER"
                    } else {
                        "INVALID_PAYLOAD"
                    };
                    send_error(socket, envelope.request_id.clone(), code, &msg).await;
                }
            }
        }
        _ => {
            send_error(
                socket,
                envelope.request_id.clone(),
                "UNKNOWN_TYPE",
                "Event type tidak dikenal server",
            )
            .await;
        }
    }
}

async fn send_error(socket: &mut WebSocket, request_id: Option<String>, code: &str, message: &str) {
    let err = WsEnvelope::new(
        "error",
        request_id,
        serde_json::json!({ "code": code, "message": message }),
    );
    let _ = socket
        .send(Message::Text(
            serde_json::to_string(&err).unwrap_or_default().into(),
        ))
        .await;
}
