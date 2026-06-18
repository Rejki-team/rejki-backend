pub mod handlers;
pub mod report_client;

use crate::application::service::ReportService;
use crate::infrastructure::pg_repository::PgReportRepository;
use auth_service_client::AuthClient;
use axum::{
    routing::{get, post},
    Router,
};
use common_auth_mw::{require_admin, require_auth};
use common_rate_limit::RateLimiter;
use notification_service_client::NotificationClient;
use sqlx::PgPool;
use std::sync::Arc;
use storage_service_client::StorageClient;

pub use report_client::ReportInProcessClient;

#[derive(Clone)]
pub struct AppState {
    pub svc: Arc<ReportService<PgReportRepository>>,
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
        let mut b = ReportService::new(Arc::new(PgReportRepository::new(pool)));
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

    // ── User route (mobile) ──────────────────────────────────────────────────
    let user = Router::new()
        .route("/", post(handlers::create_report))
        .with_state(state.clone())
        .layer(axum::middleware::from_fn_with_state(
            auth_client.clone(),
            require_auth,
        ));

    // ── Admin routes ─────────────────────────────────────────────────────────
    let admin = Router::new()
        .route("/", get(handlers::admin_list))
        .route("/export.csv", get(handlers::admin_export_csv))
        .route("/{id}", get(handlers::admin_get))
        .route("/{id}/review", post(handlers::admin_review))
        .with_state(state)
        .layer(axum::middleware::from_fn(require_admin))
        .layer(axum::middleware::from_fn_with_state(
            auth_client.clone(),
            require_auth,
        ));

    Router::new().nest("/", user).nest("/admin", admin)
}
