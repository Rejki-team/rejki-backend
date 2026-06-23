pub mod handlers;
pub mod user_client;

pub use user_client::UserInProcessClient;

use std::sync::Arc;

use axum::{routing::get, Router};
use sqlx::PgPool;

use auth_service_client::AuthClient;
use common_auth_mw::{require_auth, require_role};
use notification_service_client::NotificationClient;
use region_service_client::RegionClient;
use storage_service_client::StorageClient;

use crate::application::service::UserService;
use crate::infrastructure::PgUserRepository;

#[derive(Clone)]
pub struct AppState {
    pub user_svc: Arc<UserService<PgUserRepository>>,
    pub storage_client: Arc<dyn StorageClient>,
}

pub fn router(
    pool: PgPool,
    auth_client: Arc<dyn AuthClient>,
    region_client: Arc<dyn RegionClient>,
    storage_client: Arc<dyn StorageClient>,
    notifier: Option<Arc<dyn NotificationClient>>,
) -> Router {
    let repo = Arc::new(PgUserRepository::new(pool));
    let storage_for_svc: Option<Arc<dyn StorageClient>> = Some(storage_client.clone());
    let state = AppState {
        user_svc: Arc::new(UserService::new(
            repo,
            auth_client.clone(),
            region_client,
            storage_for_svc,
            notifier,
        )),
        storage_client,
    };

    let protected = Router::new()
        .route("/me", get(handlers::get_me))
        .route("/me", axum::routing::patch(handlers::update_me))
        .route(
            "/me/avatar",
            axum::routing::post(handlers::request_avatar_upload),
        )
        .route("/me/kyc", axum::routing::put(handlers::submit_kyc))
        .route(
            "/me/kyc/status",
            axum::routing::get(handlers::get_kyc_status),
        )
        .route(
            "/me/documents",
            axum::routing::post(handlers::request_document_upload),
        )
        .route(
            "/me/documents/commit",
            axum::routing::post(handlers::commit_document),
        )
        .route(
            "/me/documents/{kind}",
            axum::routing::get(handlers::get_document_url),
        )
        .route("/{id}", axum::routing::get(handlers::get_by_id)) // C3: ownership-protected
        .with_state(state.clone())
        .layer(axum::middleware::from_fn_with_state(
            auth_client.clone(),
            require_auth,
        ));

    // Admin router: require_auth + require_admin (default-deny).
    // Catatan urutan: `export.csv` & `{id}/review` didaftarkan sebelum `{id}` agar
    // tidak ter-shadow oleh path param `{id}`.
    let admin = Router::new()
        .route("/admin/kyc", get(handlers::admin_list_kyc))
        .route("/admin/kyc/export.csv", get(handlers::admin_export_kyc_csv))
        .route(
            "/admin/kyc/{id}/review",
            axum::routing::post(handlers::review_kyc),
        )
        .route(
            "/admin/kyc/{id}/documents/{kind}",
            get(handlers::admin_get_document),
        )
        .route("/admin/kyc/{id}", get(handlers::admin_get_kyc))
        .with_state(state.clone())
        .layer(axum::middleware::from_fn_with_state(80u8, require_role))
        .layer(axum::middleware::from_fn_with_state(
            auth_client.clone(),
            require_auth,
        ));

    // Health check tetap publik (tanpa auth).
    let public = Router::new()
        .route("/health", get(handlers::health))
        .with_state(state);

    public.merge(protected).merge(admin)
}
