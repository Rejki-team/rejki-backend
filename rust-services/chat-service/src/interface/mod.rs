pub mod handlers;
pub mod ws_handler;

use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use sqlx::PgPool;
use tokio::sync::{broadcast, Mutex};
use uuid::Uuid;

use crate::application::service::ChatService;
use crate::infrastructure::PgChatRepository;
use crate::interface::ws_handler::WsEnvelope;
use auth_service_client::AuthClient;
use common_auth_mw::require_auth;
use common_rate_limit::RateLimiter;

/// Registry broadcast channel per conversation ID. Channel dibuat saat koneksi WS pertama,
/// di-cleanup otomatis saat 0 subscriber (tokio broadcast drop).
type ConversationRooms = Arc<Mutex<HashMap<Uuid, broadcast::Sender<WsEnvelope>>>>;

#[derive(Clone)]
pub struct AppState {
    pub chat_svc: Arc<ChatService<PgChatRepository>>,
    pub auth_client: Arc<dyn AuthClient>,
    pub conversation_rooms: ConversationRooms,
}

pub fn router(
    pool: PgPool,
    auth_client: Arc<dyn AuthClient>,
    rate_limiter: Option<Arc<dyn RateLimiter>>,
) -> Router {
    let repo = Arc::new(PgChatRepository::new(pool));
    let svc = {
        let mut b = ChatService::new(repo);
        if let Some(rl) = rate_limiter {
            b = b.with_rate_limiter(rl);
        }
        b
    };
    let state = AppState {
        chat_svc: Arc::new(svc),
        auth_client: auth_client.clone(),
        conversation_rooms: Arc::new(Mutex::new(HashMap::new())),
    };

    // REST chat memerlukan login. WebSocket meng-handle auth via query token di handler.
    let protected = Router::new()
        .route("/conversations", post(handlers::get_or_create_conversation))
        .route("/conversations/{id}/messages", get(handlers::list_messages))
        .route("/conversations/{id}/messages", post(handlers::send_message))
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
