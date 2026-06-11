pub mod handlers;
use crate::application::service::IklanPekerjaanService;
use crate::infrastructure::PgIklanPekerjaanRepository;
use auth_service_client::AuthClient;
use axum::{
    routing::{delete, get, post},
    Router,
};
use common_auth_mw::{require_active_account, require_auth};
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub svc: Arc<IklanPekerjaanService<PgIklanPekerjaanRepository>>,
}

pub fn router(pool: PgPool, auth_client: Arc<dyn AuthClient>) -> Router {
    let state = AppState {
        svc: Arc::new(IklanPekerjaanService::new(Arc::new(
            PgIklanPekerjaanRepository::new(pool),
        ))),
    };

    // Route publik (tanpa login): lihat daftar & detail iklan, health.
    let public = Router::new()
        .route("/health", get(handlers::health))
        .route("/", get(handlers::list))
        .route("/{id}", get(handlers::get_by_id))
        .with_state(state.clone());

    // Route mutasi: wajib login (require_auth) + akun aktif (require_active_account).
    let protected = Router::new()
        .route("/", post(handlers::create))
        .route("/{id}", delete(handlers::delete_iklan))
        .with_state(state)
        .layer(axum::middleware::from_fn_with_state(
            auth_client.clone(),
            require_active_account,
        ))
        .layer(axum::middleware::from_fn_with_state(
            auth_client,
            require_auth,
        ));

    public.merge(protected)
}
