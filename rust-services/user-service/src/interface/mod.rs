pub mod handlers;

use std::sync::Arc;

use axum::{routing::get, Router};
use sqlx::PgPool;

use auth_service_client::AuthClient;
use region_service_client::RegionClient;
use storage_service_client::StorageClient;
use common_auth_mw::require_auth;
use crate::application::service::UserService;
use crate::infrastructure::PgUserRepository;

#[derive(Clone)]
pub struct AppState {
    pub user_svc:       Arc<UserService<PgUserRepository>>,
    pub storage_client: Arc<dyn StorageClient>,
}

pub fn router(
    pool:          PgPool,
    auth_client:   Arc<dyn AuthClient>,
    region_client: Arc<dyn RegionClient>,
    storage_client: Arc<dyn StorageClient>,
) -> Router {
    let repo = Arc::new(PgUserRepository::new(pool));
    let state = AppState {
        user_svc:       Arc::new(UserService::new(repo, auth_client.clone(), region_client)),
        storage_client,
    };

    // Profil sendiri + KYC wajib login.
    let protected = Router::new()
        .route("/me",                get(handlers::get_me))
        .route("/me",                axum::routing::patch(handlers::update_me))
        .route("/me/avatar",         axum::routing::post(handlers::request_avatar_upload))
        .route("/me/kyc",            axum::routing::put(handlers::submit_kyc))
        .route("/me/kyc/status",     axum::routing::get(handlers::get_kyc_status))
        .route("/me/documents",      axum::routing::post(handlers::request_document_upload))
        .with_state(state.clone())
        .layer(axum::middleware::from_fn_with_state(auth_client.clone(), require_auth));

    // Admin review (placeholder RBAC — menyusul di proposal Dashboard).
    let admin = Router::new()
        .route("/admin/kyc/{id}/review", axum::routing::post(handlers::review_kyc))
        .with_state(state.clone())
        .layer(axum::middleware::from_fn_with_state(auth_client.clone(), require_auth));

    // Publik: health & lihat profil pengguna lain by id.
    let public = Router::new()
        .route("/health", get(handlers::health))
        .route("/{id}",   get(handlers::get_by_id))
        .with_state(state);

    public.merge(protected).merge(admin)
}
