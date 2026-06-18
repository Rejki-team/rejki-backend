pub mod handlers;

use crate::application::service::CorporateCommsService;
use crate::infrastructure::PgCorporateArticleRepository;
use auth_service_client::AuthClient;
use axum::{
    routing::{delete, get, patch, post},
    Router,
};
use common_auth_mw::{require_admin, require_auth};
use common_rate_limit::RateLimiter;
use notification_service_client::NotificationClient;
use sqlx::PgPool;
use std::sync::Arc;
use storage_service_client::StorageClient;

#[derive(Clone)]
pub struct AppState {
    pub svc: Arc<CorporateCommsService<PgCorporateArticleRepository>>,
    pub auth_client: Arc<dyn AuthClient>,
    pub storage: Option<Arc<dyn StorageClient>>,
    pub notifier: Option<Arc<dyn NotificationClient>>,
}

pub fn router(
    pool: PgPool,
    auth_client: Arc<dyn AuthClient>,
    storage: Option<Arc<dyn StorageClient>>,
    notifier: Option<Arc<dyn NotificationClient>>,
    rate_limiter: Option<Arc<dyn RateLimiter>>,
) -> Router {
    let svc = {
        let mut b = CorporateCommsService::new(Arc::new(PgCorporateArticleRepository::new(pool)));
        if let Some(rl) = rate_limiter {
            b = b.with_rate_limiter(rl);
        }
        b
    };
    let state = AppState {
        svc: Arc::new(svc),
        auth_client: auth_client.clone(),
        storage,
        notifier,
    };

    // ── Admin routes ─────────────────────────────────────────────────────
    // Semua endpoint corporate comms diproteksi require_admin.
    let admin = Router::new()
        .route("/", get(handlers::admin_list))
        .route("/", post(handlers::admin_create))
        .route("/photo-upload", post(handlers::request_photo_upload))
        .route("/{id}", get(handlers::admin_get))
        .route("/{id}", patch(handlers::admin_update))
        .route("/{id}", delete(handlers::admin_delete))
        .with_state(state)
        .layer(axum::middleware::from_fn(require_admin))
        .layer(axum::middleware::from_fn_with_state(
            auth_client.clone(),
            require_auth,
        ));

    Router::new().nest("/", admin)
}
