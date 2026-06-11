pub mod handlers;
use crate::application::service::IklanBarangBekasService;
use crate::infrastructure::PgIklanBarangBekasRepository;
use auth_service_client::AuthClient;
use axum::{
    routing::{delete, get, patch, post},
    Router,
};
use common_auth_mw::{require_active_account, require_auth};
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub svc: Arc<IklanBarangBekasService<PgIklanBarangBekasRepository>>,
}

pub fn router(pool: PgPool, auth_client: Arc<dyn AuthClient>) -> Router {
    let state = AppState {
        svc: Arc::new(IklanBarangBekasService::new(Arc::new(
            PgIklanBarangBekasRepository::new(pool),
        ))),
    };

    let public = Router::new()
        .route("/health", get(handlers::health))
        .route("/", get(handlers::list))
        .route("/{id}", get(handlers::get_by_id))
        .with_state(state.clone());

    let protected = Router::new()
        .route("/", post(handlers::create))
        .route("/{id}", delete(handlers::delete_iklan))
        .route("/{id}/sold", patch(handlers::mark_sold))
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
