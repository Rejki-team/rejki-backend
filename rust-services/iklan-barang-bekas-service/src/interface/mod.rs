pub mod handlers;
use crate::application::service::IklanBarangBekasService;
use crate::infrastructure::PgIklanBarangBekasRepository;
use auth_service_client::AuthClient;
use axum::{
    routing::{delete, get, patch, post},
    Router,
};
use common_auth_mw::{require_active_account, require_admin, require_auth};
use common_rate_limit::RateLimiter;
use notification_service_client::NotificationClient;
use sqlx::PgPool;
use std::sync::Arc;
use storage_service_client::StorageClient;

#[derive(Clone)]
pub struct AppState {
    pub svc: Arc<IklanBarangBekasService<PgIklanBarangBekasRepository>>,
    pub storage: Option<Arc<dyn StorageClient>>,
    pub notifier: Option<Arc<dyn NotificationClient>>,
    pub auth_client: Option<Arc<dyn AuthClient>>,
}

pub fn router(
    pool: PgPool,
    auth_client: Arc<dyn AuthClient>,
    storage: Option<Arc<dyn StorageClient>>,
    notifier: Option<Arc<dyn NotificationClient>>,
    rate_limiter: Option<Arc<dyn RateLimiter>>,
) -> Router {
    let svc = {
        let mut b = IklanBarangBekasService::new(Arc::new(PgIklanBarangBekasRepository::new(pool)));
        if let Some(rl) = rate_limiter {
            b = b.with_rate_limiter(rl);
        }
        b
    };
    let state = AppState {
        svc: Arc::new(svc),
        storage,
        notifier,
        auth_client: Some(auth_client.clone()),
    };

    let public = Router::new()
        .route("/health", get(handlers::health))
        .route("/", get(handlers::list))
        .route("/{id}", get(handlers::get_by_id))
        .with_state(state.clone());

    let protected = Router::new()
        .route("/", post(handlers::create))
        .route("/{id}", delete(handlers::delete_iklan))
        .route("/{id}/taken", patch(handlers::mark_taken))
        .with_state(state.clone())
        .layer(axum::middleware::from_fn_with_state(
            auth_client.clone(),
            require_active_account,
        ))
        .layer(axum::middleware::from_fn_with_state(
            auth_client.clone(),
            require_auth,
        ));

    let admin = Router::new()
        .route("/", get(handlers::admin_list))
        .route("/export.csv", get(handlers::admin_export_csv))
        .route("/suspend/evidence", post(handlers::admin_request_evidence))
        .route("/suspend", post(handlers::admin_suspend))
        .with_state(state)
        .layer(axum::middleware::from_fn(require_admin))
        .layer(axum::middleware::from_fn_with_state(
            auth_client.clone(),
            require_active_account,
        ))
        .layer(axum::middleware::from_fn_with_state(
            auth_client,
            require_auth,
        ));

    public
        .merge(protected)
        .merge(Router::new().nest("/admin", admin))
}
