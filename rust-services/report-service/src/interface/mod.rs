pub mod handlers;
pub mod report_client;

use crate::application::service::ReportService;
use crate::infrastructure::pg_repository::PgReportRepository;
use auth_service_client::AuthClient;
use axum::{
    routing::{get, post},
    Router,
};
use common_auth_mw::{require_auth, require_role};
use common_rate_limit::RateLimiter;
use iklan_barang_bekas_service_client::IklanBarangBekasClient;
use iklan_pekerja_service_client::IklanPekerjaClient;
use iklan_pekerjaan_service_client::IklanPekerjaanClient;
use notification_service_client::NotificationClient;
use region_service_client::RegionClient;
use sqlx::PgPool;
use std::sync::Arc;
use storage_service_client::StorageClient;
use user_service_client::UserClient;

pub use report_client::ReportInProcessClient;

#[derive(Clone)]
pub struct AppState {
    pub svc: Arc<ReportService<PgReportRepository>>,
    pub auth_client: Arc<dyn AuthClient>,
    pub storage: Option<Arc<dyn StorageClient>>,
    pub notifier: Option<Arc<dyn NotificationClient>>,
    /// P9.1 (Kelompok 6 Q9) — dipilih runtime berdasar `target_ad_type` aduan
    /// saat endpoint approve-and-suspend dipanggil.
    pub iklan_pekerjaan_client: Option<Arc<dyn IklanPekerjaanClient>>,
    pub iklan_pekerja_client: Option<Arc<dyn IklanPekerjaClient>>,
    pub iklan_barang_bekas_client: Option<Arc<dyn IklanBarangBekasClient>>,
}

/// Dependency untuk `router()` — Zero Too Many Arguments.
pub struct RouterDeps {
    pub pool: PgPool,
    pub auth_client: Arc<dyn AuthClient>,
    pub storage: Option<Arc<dyn StorageClient>>,
    pub notifier: Option<Arc<dyn NotificationClient>>,
    pub rate_limiter: Option<Arc<dyn RateLimiter>>,
    pub user_client: Option<Arc<dyn UserClient>>,
    pub region_client: Option<Arc<dyn RegionClient>>,
    pub iklan_pekerjaan_client: Option<Arc<dyn IklanPekerjaanClient>>,
    pub iklan_pekerja_client: Option<Arc<dyn IklanPekerjaClient>>,
    pub iklan_barang_bekas_client: Option<Arc<dyn IklanBarangBekasClient>>,
}

pub fn router(deps: RouterDeps) -> Router {
    let RouterDeps {
        pool,
        auth_client,
        storage,
        notifier,
        rate_limiter,
        user_client,
        region_client,
        iklan_pekerjaan_client,
        iklan_pekerja_client,
        iklan_barang_bekas_client,
    } = deps;

    let svc = {
        let mut b = ReportService::new(Arc::new(PgReportRepository::new(pool)));
        if let Some(rl) = rate_limiter {
            b = b.with_rate_limiter(rl);
        }
        if let Some(uc) = user_client {
            b = b.with_user_client(uc);
        }
        if let Some(rc) = region_client {
            b = b.with_region_client(rc);
        }
        b
    };
    let state = AppState {
        svc: Arc::new(svc),
        auth_client: auth_client.clone(),
        storage,
        notifier,
        iklan_pekerjaan_client,
        iklan_pekerja_client,
        iklan_barang_bekas_client,
    };

    // ── User routes (mobile, 2 jalur — P1.4) ──────────────────────────────────
    let user = Router::new()
        .route("/laporkan-iklan", post(handlers::create_laporkan_iklan))
        .route(
            "/pelaporan-masalah",
            post(handlers::create_pelaporan_masalah),
        )
        .with_state(state.clone())
        .layer(axum::middleware::from_fn_with_state(
            auth_client.clone(),
            require_auth,
        ));

    // ── Admin routes ─────────────────────────────────────────────────────────
    let admin = Router::new()
        .route("/", get(handlers::admin_list))
        .route("/export.csv", get(handlers::admin_export_csv))
        .route("/{id}", get(handlers::admin_get))
        .route("/{id}/review", post(handlers::admin_review))
        .route(
            "/{id}/approve-and-suspend",
            post(handlers::admin_approve_and_suspend),
        )
        .with_state(state)
        .layer(axum::middleware::from_fn_with_state(60u8, require_role))
        .layer(axum::middleware::from_fn_with_state(
            auth_client.clone(),
            require_auth,
        ));

    user.merge(Router::new().nest("/admin", admin))
}
