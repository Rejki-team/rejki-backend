pub mod handlers;
pub mod region_client;

use std::sync::Arc;

use axum::{routing::get, Router};
use serde::Deserialize;
use sqlx::PgPool;

use crate::application::service::RegionService;
use crate::infrastructure::PgRegionRepository;
pub use region_client::RegionInProcessClient;

#[derive(Clone)]
pub struct AppState {
    pub region_svc: Arc<RegionService<PgRegionRepository>>,
}

/// Query parameter untuk lookup berdasarkan induk.
#[derive(Deserialize)]
pub struct ParentQuery {
    pub province_id: Option<String>,
    pub regency_id: Option<String>,
    pub district_id: Option<String>,
}

pub fn router(pool: PgPool) -> Router {
    let repo = Arc::new(PgRegionRepository::new(pool));
    let state = AppState {
        region_svc: Arc::new(RegionService::new(repo)),
    };

    Router::new()
        .route("/provinces", get(handlers::provinces))
        .route("/regencies", get(handlers::regencies))
        .route("/districts", get(handlers::districts))
        .route("/villages", get(handlers::villages))
        .with_state(state)
}
