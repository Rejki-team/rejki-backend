pub mod handlers;
pub mod ws_handler;

use std::sync::Arc;

use axum::{
    routing::{get, patch, post},
    Router,
};
use dashmap::DashMap;
use sqlx::PgPool;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::application::service::ChatService;
use crate::infrastructure::PgChatRepository;
use crate::interface::ws_handler::WsEnvelope;
use auth_service_client::AuthClient;
use common_auth_mw::require_auth;
use common_rate_limit::RateLimiter;
use common_scheduler::SchedulerClient;
use storage_service_client::StorageClient;
use user_service_client::UserClient;

/// Registry broadcast channel per conversation ID. Channel dibuat saat koneksi WS pertama,
/// di-cleanup otomatis saat 0 subscriber. `DashMap` (bukan `Mutex<HashMap>`, Audit Finding #1
/// 2026-09-22) — lock ter-shard per-entry, bukan satu lock global untuk SELURUH percakapan;
/// connect/disconnect/kirim pesan di percakapan berbeda tidak lagi saling serialisasi.
type ConversationRooms = Arc<DashMap<Uuid, broadcast::Sender<WsEnvelope>>>;

/// Hitung koneksi WS aktif per user (Audit Finding #3 2026-09-22) — dasar pembatasan
/// `MAX_WS_CONNECTIONS_PER_USER` di `ws_handler::ws_handler`.
type WsConnectionCounts = Arc<DashMap<Uuid, u32>>;

#[derive(Clone)]
pub struct AppState {
    pub chat_svc: Arc<ChatService<PgChatRepository>>,
    pub auth_client: Arc<dyn AuthClient>,
    pub conversation_rooms: ConversationRooms,
    pub ws_connection_counts: WsConnectionCounts,
}

pub fn router(
    pool: PgPool,
    auth_client: Arc<dyn AuthClient>,
    rate_limiter: Option<Arc<dyn RateLimiter>>,
    scheduler_client: Option<Arc<SchedulerClient>>,
    storage_client: Option<Arc<dyn StorageClient>>,
    user_client: Option<Arc<dyn UserClient>>,
) -> Router {
    let repo = Arc::new(PgChatRepository::new(pool));
    let svc = {
        let mut b = ChatService::new(repo);
        if let Some(rl) = rate_limiter {
            b = b.with_rate_limiter(rl);
        }
        if let Some(sc) = scheduler_client {
            b = b.with_scheduler_client(sc);
        }
        if let Some(sc) = storage_client {
            b = b.with_storage_client(sc);
        }
        if let Some(uc) = user_client {
            b = b.with_user_client(uc);
        }
        b
    };
    let state = AppState {
        chat_svc: Arc::new(svc),
        auth_client: auth_client.clone(),
        conversation_rooms: Arc::new(DashMap::new()),
        ws_connection_counts: Arc::new(DashMap::new()),
    };

    // REST chat memerlukan login. WebSocket meng-handle auth via query token di handler.
    let protected = Router::new()
        .route(
            "/conversations",
            get(handlers::list_conversations).post(handlers::get_or_create_conversation),
        )
        .route("/conversations/{id}/messages", get(handlers::list_messages))
        .route("/conversations/{id}/messages", post(handlers::send_message))
        .route(
            "/conversations/{id}/akhiri",
            patch(handlers::end_conversation),
        )
        .route("/conversations/{id}/read", patch(handlers::mark_read))
        .route(
            "/photo-upload-permission",
            post(handlers::request_photo_upload),
        )
        .with_state(state.clone())
        .layer(axum::middleware::from_fn_with_state(
            auth_client,
            require_auth,
        ));

    let public = Router::new()
        .route("/ws", get(ws_handler::ws_handler))
        .with_state(state);

    public.merge(protected)
}
