pub mod handlers;

use crate::application::service::IklanPelatihanService;
use crate::infrastructure::PgIklanPelatihanRepository;
use auth_service_client::AuthClient;
use axum::{
    routing::{delete, get, patch, post},
    Router,
};
use common_auth_mw::{require_active_account, require_auth, require_role};
use common_rate_limit::RateLimiter;
use notification_service_client::NotificationClient;
use region_service_client::RegionClient;
use sqlx::PgPool;
use std::sync::Arc;
use storage_service_client::StorageClient;

#[derive(Clone)]
pub struct AppState {
    pub svc: Arc<IklanPelatihanService<PgIklanPelatihanRepository>>,
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
    region_client: Option<Arc<dyn RegionClient>>,
) -> Router {
    let svc = {
        let mut b = IklanPelatihanService::new(Arc::new(PgIklanPelatihanRepository::new(pool)));
        if let Some(rl) = rate_limiter {
            b = b.with_rate_limiter(rl);
        }
        if let Some(rc) = region_client {
            b = b.with_region_client(rc);
        }
        b
    };
    let state = AppState {
        svc: Arc::new(svc),
        storage,
        notifier,
        auth_client: Some(auth_client.clone()),
    };

    // ── Public ────────────────────────────────────────────────────────
    let public = Router::new()
        .route("/health", get(handlers::health))
        .route("/", get(handlers::list))
        .route("/{id}", get(handlers::get_by_id))
        .with_state(state.clone());

    // ── Protected (auth user) ─────────────────────────────────────────
    let protected = Router::new()
        .route("/", post(handlers::create))
        .route("/{id}", patch(handlers::update))
        .route("/{id}", delete(handlers::delete_iklan))
        // Enrollment
        .route("/{id}/enroll", post(handlers::create_enrollment))
        .route(
            "/enrollment/evidence",
            post(handlers::request_enroll_evidence),
        )
        .route(
            "/enrollments/{id}/commit-bukti",
            post(handlers::commit_enrollment_bukti),
        )
        // Badge
        .route("/{id}/badge", post(handlers::create_badge))
        .route("/badge/evidence", post(handlers::request_badge_evidence))
        .route(
            "/badges/{id}/commit-sertifikat",
            post(handlers::commit_badge_sertifikat),
        )
        .with_state(state.clone())
        .layer(axum::middleware::from_fn_with_state(
            auth_client.clone(),
            require_active_account,
        ))
        .layer(axum::middleware::from_fn_with_state(
            auth_client.clone(),
            require_auth,
        ));

    // ── Admin ─────────────────────────────────────────────────────────
    let admin = Router::new()
        // Pelatihan management
        .route("/pelatihan", post(handlers::admin_create_pelatihan))
        .route("/pelatihan", get(handlers::admin_pelatihan_list))
        .route("/pelatihan/{id}", patch(handlers::admin_update_pelatihan))
        .route("/pelatihan/{id}", delete(handlers::admin_cancel_pelatihan))
        .route(
            "/pelatihan/{id}/review",
            post(handlers::admin_review_pelatihan),
        )
        .route(
            "/pelatihan/export.csv",
            get(handlers::admin_pelatihan_export_csv),
        )
        // Moderation / suspension (existing)
        .route("/", get(handlers::admin_list))
        .route("/export.csv", get(handlers::admin_export_csv))
        .route("/suspend/evidence", post(handlers::admin_request_evidence))
        .route("/suspend", post(handlers::admin_suspend))
        // Enrollment management
        .route("/enrollments", get(handlers::admin_enrollment_list))
        .route("/enrollments/{id}", get(handlers::admin_enrollment_detail))
        .route(
            "/enrollments/{id}/review",
            post(handlers::admin_enrollment_review),
        )
        .route(
            "/enrollments/export.csv",
            get(handlers::admin_enrollment_export_csv),
        )
        // Badge management
        .route("/badges", get(handlers::admin_badge_list))
        .route("/badges/{id}", get(handlers::admin_badge_detail))
        .route("/badges/{id}/review", post(handlers::admin_badge_review))
        .route("/badges/export.csv", get(handlers::admin_badge_export_csv))
        .with_state(state)
        .layer(axum::middleware::from_fn_with_state(80u8, require_role))
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
