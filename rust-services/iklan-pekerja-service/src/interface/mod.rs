pub mod handlers;
use crate::application::service::IklanPekerjaService;
use crate::infrastructure::PgIklanPekerjaRepository;
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
    pub svc: Arc<IklanPekerjaService<PgIklanPekerjaRepository>>,
}

pub fn router(pool: PgPool, auth_client: Arc<dyn AuthClient>) -> Router {
    let state = AppState {
        svc: Arc::new(IklanPekerjaService::new(Arc::new(
            PgIklanPekerjaRepository::new(pool),
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
