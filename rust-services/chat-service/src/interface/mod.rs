pub mod handlers;
pub mod ws_handler;

use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use sqlx::PgPool;

use crate::application::service::ChatService;
use crate::infrastructure::PgChatRepository;
use auth_service_client::AuthClient;
use common_auth_mw::require_auth;

#[derive(Clone)]
pub struct AppState {
    pub chat_svc: Arc<ChatService<PgChatRepository>>,
    pub auth_client: Arc<dyn AuthClient>,
}

pub fn router(pool: PgPool, auth_client: Arc<dyn AuthClient>) -> Router {
    let repo = Arc::new(PgChatRepository::new(pool));
    let state = AppState {
        chat_svc: Arc::new(ChatService::new(repo)),
        auth_client: auth_client.clone(),
    };

    // REST chat memerlukan login. WebSocket meng-handle auth via query token di handler.
    let protected = Router::new()
        .route("/conversations", post(handlers::get_or_create_conversation))
        .route("/conversations/{id}/messages", get(handlers::list_messages))
        .route("/conversations/{id}/messages", post(handlers::send_message))
        .with_state(state.clone())
        .layer(axum::middleware::from_fn_with_state(auth_client, require_auth));

    let public = Router::new()
        .route("/ws", get(ws_handler::ws_handler))
        .with_state(state);

    public.merge(protected)
}
