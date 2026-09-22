pub mod handlers;
use crate::application::service::RatingService;
use crate::infrastructure::PgRatingRepository;
use auth_service_client::AuthClient;
use axum::{
    routing::{get, post},
    Router,
};
use common_auth_mw::{require_active_account, require_auth};
use iklan_pekerjaan_service_client::IklanPekerjaanClient;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub svc: Arc<RatingService<PgRatingRepository>>,
}

pub fn router(
    pool: PgPool,
    auth_client: Arc<dyn AuthClient>,
    iklan_pekerjaan_client: Option<Arc<dyn IklanPekerjaanClient>>,
) -> Router {
    let svc = {
        let mut b = RatingService::new(Arc::new(PgRatingRepository::new(pool)));
        if let Some(c) = iklan_pekerjaan_client {
            b = b.with_iklan_pekerjaan_client(c);
        }
        b
    };
    let state = AppState { svc: Arc::new(svc) };

    let public = Router::new()
        .route("/health", get(handlers::health))
        .route("/profil/{user_id}", get(handlers::get_aggregate))
        .with_state(state.clone());

    let protected = Router::new()
        .route("/", post(handlers::create_rating))
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
