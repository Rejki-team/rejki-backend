pub mod handlers;
pub mod refresh;

use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use common_auth_mw::{require_auth, require_role};
use sqlx::PgPool;

use crate::application::service::InsightsService;
use crate::infrastructure::pg_repository::PgInsightsRepository;

#[derive(Clone)]
pub struct AppState {
    pub svc: Arc<InsightsService<PgInsightsRepository>>,
}

pub fn router(pool: PgPool, auth_client: Arc<dyn auth_service_client::AuthClient>) -> Router {
    let repo = Arc::new(PgInsightsRepository::new(pool));
    let svc = Arc::new(InsightsService::new(repo));
    let state = AppState { svc: svc.clone() };

    // Spawn auto-refresh scheduler
    let refresh_svc = svc.clone();
    tokio::spawn(async move {
        refresh::start_auto_refresh(refresh_svc).await;
    });

    Router::new()
        .route("/users", get(handlers::get_user_stats))
        .route("/iklan", get(handlers::get_iklan_stats))
        .route("/geo", get(handlers::get_geo_stats))
        .route("/engagement", get(handlers::get_engagement_stats))
        .route("/canvassing", get(handlers::get_canvassing))
        .route("/refresh", post(handlers::refresh))
        .with_state(state)
        .layer(axum::middleware::from_fn_with_state(90u8, require_role))
        .layer(axum::middleware::from_fn_with_state(
            auth_client,
            require_auth,
        ))
}
