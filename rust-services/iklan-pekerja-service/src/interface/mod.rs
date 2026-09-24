pub mod handlers;
use crate::application::service::IklanPekerjaService;
use crate::infrastructure::PgIklanPekerjaRepository;
use auth_service_client::AuthClient;
use axum::{
    routing::{delete, get, patch, post},
    Router,
};
use common_auth_mw::{require_active_account, require_auth, require_role};
use common_geocoding::GeocodingClient;
use common_rate_limit::RateLimiter;
use notification_service_client::NotificationClient;
use region_service_client::RegionClient;
use sqlx::PgPool;
use std::sync::Arc;
use storage_service_client::StorageClient;
use user_service_client::UserClient;

#[derive(Clone)]
pub struct AppState {
    pub svc: Arc<IklanPekerjaService<PgIklanPekerjaRepository>>,
    pub storage: Option<Arc<dyn StorageClient>>,
    pub notifier: Option<Arc<dyn NotificationClient>>,
    pub auth_client: Option<Arc<dyn AuthClient>>,
}

/// Dependency bundle untuk `router()` — dibungkus struct (bukan 8 argumen lepas)
/// per CLAUDE.md §4.7 Zero Too Many Arguments.
pub struct RouterDeps {
    pub pool: PgPool,
    pub auth_client: Arc<dyn AuthClient>,
    pub storage: Option<Arc<dyn StorageClient>>,
    pub notifier: Option<Arc<dyn NotificationClient>>,
    pub rate_limiter: Option<Arc<dyn RateLimiter>>,
    pub region_client: Option<Arc<dyn RegionClient>>,
    pub user_client: Option<Arc<dyn UserClient>>,
    pub geocoding_client: Option<Arc<dyn GeocodingClient>>,
}

pub fn router(deps: RouterDeps) -> Router {
    let RouterDeps {
        pool,
        auth_client,
        storage,
        notifier,
        rate_limiter,
        region_client,
        user_client,
        geocoding_client,
    } = deps;
    let svc = {
        let mut b = IklanPekerjaService::new(Arc::new(PgIklanPekerjaRepository::new(pool)));
        if let Some(rl) = rate_limiter {
            b = b.with_rate_limiter(rl);
        }
        if let Some(rc) = region_client {
            b = b.with_region_client(rc);
        }
        if let Some(uc) = user_client {
            b = b.with_user_client(uc);
        }
        if let Some(gc) = geocoding_client {
            b = b.with_geocoding_client(gc);
        }
        b
    };
    let state = AppState {
        svc: Arc::new(svc),
        storage,
        notifier,
        auth_client: Some(auth_client.clone()),
    };

    let public = Router::new()
        .route("/health", get(handlers::health))
        .route("/", get(handlers::list))
        .route("/{id}", get(handlers::get_by_id))
        .with_state(state.clone());

    let protected = Router::new()
        .route("/", post(handlers::create))
        .route("/{id}", delete(handlers::delete_iklan))
        .route("/{id}", patch(handlers::update))
        .with_state(state.clone())
        .layer(axum::middleware::from_fn_with_state(
            auth_client.clone(),
            require_active_account,
        ))
        .layer(axum::middleware::from_fn_with_state(
            auth_client.clone(),
            require_auth,
        ));

    let admin = Router::new()
        .route("/", get(handlers::admin_list))
        .route("/export.csv", get(handlers::admin_export_csv))
        .route("/suspend/evidence", post(handlers::admin_request_evidence))
        .route("/suspend", post(handlers::admin_suspend))
        .route(
            "/{id}/sensitive/{kind}",
            get(handlers::admin_reveal_sensitive),
        )
        .route("/{id}", get(handlers::admin_get_detail))
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
