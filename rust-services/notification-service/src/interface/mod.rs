pub mod handlers;

use std::sync::Arc;

use axum::{
    routing::{get, patch, post},
    Router,
};
use sqlx::PgPool;

use crate::application::service::NotificationService;
use crate::infrastructure::{PgNotificationRepository, RedisPublisher};
use auth_service_client::AuthClient;
use common_auth_mw::require_auth;

#[derive(Clone)]
pub struct AppState {
    pub notif_svc: Arc<NotificationService<PgNotificationRepository>>,
}

pub fn router(pool: PgPool, auth_client: Arc<dyn AuthClient>) -> Router {
    let repo = Arc::new(PgNotificationRepository::new(pool));

    let svc = if let Ok(redis_url) = std::env::var("REDIS_URL") {
        match RedisPublisher::new(&redis_url) {
            Ok(pub_) => {
                tracing::info!("Redis publisher connected");
                NotificationService::with_publisher(repo, Arc::new(pub_))
            }
            Err(e) => {
                tracing::warn!(error = ?e, "Redis publisher unavailable — notifications will not be pushed");
                NotificationService::new(repo)
            }
        }
    } else {
        tracing::warn!("REDIS_URL not set — push notifications disabled");
        NotificationService::new(repo)
    };

    let state = AppState {
        notif_svc: Arc::new(svc),
    };

    // Semua endpoint notifikasi memerlukan login.
    Router::new()
        .route("/", get(handlers::list_my_notifications))
        .route("/send", post(handlers::send))
        .route("/{id}/read", patch(handlers::mark_read))
        .with_state(state)
        .layer(axum::middleware::from_fn_with_state(
            auth_client,
            require_auth,
        ))
}
