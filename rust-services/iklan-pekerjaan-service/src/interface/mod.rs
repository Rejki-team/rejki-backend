pub mod handlers;
use crate::application::service::IklanPekerjaanService;
use crate::infrastructure::PgIklanPekerjaanRepository;
use auth_service_client::AuthClient;
use axum::{
    routing::{delete, get, patch, post},
    Router,
};
use chat_service_client::ChatClient;
use common_auth_mw::{require_active_account, require_auth, require_role};
use common_geocoding::GeocodingClient;
use common_rate_limit::RateLimiter;
use iklan_pekerja_service_client::IklanPekerjaClient;
use notification_service_client::NotificationClient;
use region_service_client::RegionClient;
use sqlx::PgPool;
use std::sync::Arc;
use storage_service_client::StorageClient;

#[derive(Clone)]
pub struct AppState {
    pub svc: Arc<IklanPekerjaanService<PgIklanPekerjaanRepository>>,
    pub storage: Option<Arc<dyn StorageClient>>,
    pub notifier: Option<Arc<dyn NotificationClient>>,
    pub auth_client: Option<Arc<dyn AuthClient>>,
}

/// Dependency `router()` — dibungkus struct (bukan argumen lepas) untuk menghindari
/// `clippy::too_many_arguments` (CLAUDE.md §4.7), sama seperti `iklan-pelatihan-service`.
pub struct RouterDeps {
    pub pool: PgPool,
    pub auth_client: Arc<dyn AuthClient>,
    pub storage: Option<Arc<dyn StorageClient>>,
    pub notifier: Option<Arc<dyn NotificationClient>>,
    pub rate_limiter: Option<Arc<dyn RateLimiter>>,
    pub region_client: Option<Arc<dyn RegionClient>>,
    pub geocoding_client: Option<Arc<dyn GeocodingClient>>,
    pub iklan_pekerja_client: Option<Arc<dyn IklanPekerjaClient>>,
    pub chat_client: Option<Arc<dyn ChatClient>>,
}

pub fn router(deps: RouterDeps) -> Router {
    let auth_client = deps.auth_client;
    let svc = {
        let mut b =
            IklanPekerjaanService::new(Arc::new(PgIklanPekerjaanRepository::new(deps.pool)));
        if let Some(rl) = deps.rate_limiter {
            b = b.with_rate_limiter(rl);
        }
        if let Some(rc) = deps.region_client {
            b = b.with_region_client(rc);
        }
        if let Some(gc) = deps.geocoding_client {
            b = b.with_geocoding_client(gc);
        }
        if let Some(pc) = deps.iklan_pekerja_client {
            b = b.with_iklan_pekerja_client(pc);
        }
        if let Some(cc) = deps.chat_client {
            b = b.with_chat_client(cc);
        }
        b
    };
    let state = AppState {
        svc: Arc::new(svc),
        storage: deps.storage,
        notifier: deps.notifier,
        auth_client: Some(auth_client.clone()),
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
        .route("/saya", get(handlers::list_my_jobs))
        .route("/{id}", delete(handlers::delete_iklan))
        .route("/{id}", patch(handlers::update_iklan))
        // ── Lamaran (F-3, Kelompok 3 Phase 1) ──
        .route("/lamaran/saya", get(handlers::list_lamaran_saya))
        .route("/{id}/lamar", post(handlers::lamar))
        .route("/{id}/lamaran", get(handlers::list_lamaran_for_iklan))
        .route(
            "/{id}/lamaran/{lamaran_id}",
            patch(handlers::review_lamaran),
        )
        .route(
            "/{id}/lamaran/{lamaran_id}/mulai-bekerja",
            post(handlers::mulai_bekerja),
        )
        .route(
            "/{id}/lamaran/{lamaran_id}/tandai-selesai",
            post(handlers::tandai_selesai),
        )
        .route(
            "/{id}/lamaran/{lamaran_id}/batalkan",
            patch(handlers::batalkan_lamaran),
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

    // Route admin: wajib login + akun aktif + role admin.
    let admin = Router::new()
        .route("/", get(handlers::admin_list))
        .route("/export.csv", get(handlers::admin_export_csv))
        .route("/suspend/evidence", post(handlers::admin_request_evidence))
        .route("/suspend", post(handlers::admin_suspend))
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
