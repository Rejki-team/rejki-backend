use std::sync::Arc;

use axum::{http::StatusCode, response::Json, routing::get, Router};
use serde_json::json;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};

mod openapi;
mod rate_limit_middleware;

#[tokio::main]
async fn main() {
    // ── 1. Load APP_ENV — fail-fast jika tidak di-set ─────────────────────────
    let app_env = common_config::load_app_env();

    // ── 2. Load semua env var dari .env (development only) + validasi ─────────
    let cfg = common_config::AppConfig::from_env(app_env);

    // ── 3. Init tracing ───────────────────────────────────────────────────────
    common_tracing::init_tracing();

    tracing::info!(
        service = %cfg.service_name,
        env     = ?cfg.app_env,
        "starting rejki-app"
    );

    // ── 4. Database pool (shared — schema terisolasi per service) ─────────────
    let pool = sqlx::PgPool::connect(&cfg.database_url)
        .await
        .expect("gagal connect ke PostgreSQL");

    tracing::info!("database connected");

    // ── 5. JwtService ─────────────────────────────────────────────────────────
    let private_pem = std::fs::read_to_string(
        std::env::var("JWT_PRIVATE_KEY_PATH").unwrap_or_else(|_| "./keys/private.pem".into()),
    )
    .expect("JWT_PRIVATE_KEY_PATH tidak ditemukan — generate: openssl genrsa -out keys/private.pem 2048");

    let public_pem = std::fs::read_to_string(
        std::env::var("JWT_PUBLIC_KEY_PATH").unwrap_or_else(|_| "./keys/public.pem".into()),
    )
    .expect("JWT_PUBLIC_KEY_PATH tidak ditemukan");

    let access_ttl: i64 = std::env::var("JWT_ACCESS_TTL_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(900);

    let jwt = Arc::new(
        auth_service::JwtService::from_files(&private_pem, &public_pem, access_ttl)
            .expect("gagal inisiasi JwtService"),
    );

    // ── 6. AuthClient — in-process implementation ─────────────────────────────
    // Repo auth dibuat sekali & di-share ke router auth dan AuthInProcessClient,
    // agar status akun punya satu sumber kebenaran (lihat K2 / spec account-status-lifecycle).
    let auth_repo = Arc::new(auth_service::PgAuthRepository::new(pool.clone()));
    let auth_client: Arc<dyn auth_service::AuthClient> = Arc::new(
        auth_service::AuthInProcessClient::new(jwt.clone(), auth_repo.clone()),
    );

    let notifier: Option<Arc<dyn notification_service::NotificationClient>> =
        match std::env::var("REDIS_URL")
            .ok()
            .and_then(|u| notification_service::NotificationPublisher::new(&u).ok())
        {
            Some(p) => Some(Arc::new(p)),
            None => {
                tracing::warn!("REDIS_URL tidak diset — email OTP tidak akan dikirim");
                None
            }
        };

    // RegionClient — implementasi in-process untuk user-service (validasi wilayah KYC).
    let region_client: Arc<dyn region_service::RegionClient> = {
        let repo = Arc::new(region_service::PgRegionRepository::new(pool.clone()));
        let svc = Arc::new(region_service::RegionService::new(repo));
        Arc::new(region_service::RegionInProcessClient::new(svc))
    };

    // StorageClient — in-process (MinIO/S3 via presigned URL). MinioStorage di-init
    // dari env; bila env tidak diset, request_upload mengembalikan Unavailable.
    let storage_client: Arc<dyn storage_service::StorageClient> =
        Arc::new(storage_service::StorageInProcessClient::new().await);

    // UserClient — in-process untuk suspend permanen → purge dokumen KYC (D4).
    // Dibangun dari UserService yang sama dengan user-service router (satu instance,
    // satu sumber kebenaran profil/dokumen).
    let user_client: Arc<dyn user_service_client::UserClient> = {
        let user_repo = Arc::new(user_service::PgUserRepository::new(pool.clone()));
        let user_svc = Arc::new(user_service::UserService::new(
            user_repo,
            auth_client.clone(),
            region_client.clone(),
            Some(storage_client.clone()),
            notifier.clone(),
        ));
        Arc::new(user_service::UserInProcessClient::new(user_svc))
    };

    // ── 7. Rate limiter (shared — Redis Lua atomik, fail-open) ────────────────
    let rate_limiter: Option<Arc<dyn common_rate_limit::RateLimiter>> =
        if std::env::var("REDIS_URL").is_ok() || cfg!(test) {
            Some(Arc::new(common_rate_limit::OtpRateLimiter::from_env()))
        } else {
            tracing::warn!("REDIS_URL tidak diset — rate limiter non-aktif");
            None
        };

    // ── 8. Build router ───────────────────────────────────────────────────────
    let api_v1 = Router::new()
        .nest(
            "/auth",
            auth_service::router_with_deps_ex(
                jwt.clone(),
                auth_repo.clone(),
                auth_client.clone(),
                notifier.clone(),
                Some(storage_client.clone()),
                Some(user_client.clone()),
            ),
        )
        .nest("/regions", region_service::router(pool.clone()))
        .nest(
            "/users",
            user_service::router(
                pool.clone(),
                auth_client.clone(),
                region_client.clone(),
                storage_client.clone(),
                notifier.clone(),
            ),
        )
        .nest(
            "/chat",
            chat_service::router(pool.clone(), auth_client.clone(), rate_limiter.clone()),
        )
        .nest(
            "/notif",
            notification_service::router(pool.clone(), auth_client.clone(), rate_limiter.clone()),
        )
        .nest(
            "/pekerjaan",
            iklan_pekerjaan_service::router(
                pool.clone(),
                auth_client.clone(),
                Some(storage_client.clone()),
                notifier.clone(),
                rate_limiter.clone(),
                Some(region_client.clone()),
            ),
        )
        .nest(
            "/pekerja",
            iklan_pekerja_service::router(
                pool.clone(),
                auth_client.clone(),
                Some(storage_client.clone()),
                notifier.clone(),
                rate_limiter.clone(),
                Some(region_client.clone()),
            ),
        )
        .nest(
            "/barang",
            iklan_barang_bekas_service::router(
                pool.clone(),
                auth_client.clone(),
                Some(storage_client.clone()),
                notifier.clone(),
                rate_limiter.clone(),
                Some(region_client.clone()),
            ),
        )
        .nest(
            "/pelatihan",
            iklan_pelatihan_service::router(
                pool.clone(),
                auth_client.clone(),
                Some(storage_client.clone()),
                notifier.clone(),
                rate_limiter.clone(),
                Some(region_client.clone()),
            ),
        )
        .nest(
            "/admin/articles",
            corporate_comms_service::router(
                pool.clone(),
                auth_client.clone(),
                Some(storage_client.clone()),
                notifier.clone(),
                rate_limiter.clone(),
            ),
        )
        .nest(
            "/reports",
            report_service::router(
                pool.clone(),
                auth_client.clone(),
                Some(storage_client.clone()),
                notifier.clone(),
                rate_limiter.clone(),
            ),
        );

    let mut app = Router::new()
        .route("/health", get(health_handler))
        .nest("/api/v1", api_v1);

    // ── Swagger UI — HANYA development ────────────────────────────────────────
    // Di production rute ini TIDAK dipasang sama sekali (404), sehingga baik UI
    // maupun dokumen `openapi.json` tidak terekspos (design D2/D6).
    if cfg.app_env.is_development() {
        use utoipa::OpenApi;
        use utoipa_swagger_ui::SwaggerUi;
        app = app.merge(
            SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", openapi::ApiDoc::openapi()),
        );
        tracing::info!("Swagger UI aktif (development) — /swagger-ui");
    }

    let rl_state = rate_limit_middleware::RateLimitState {
        limiter: rate_limiter.clone(),
    };

    let app = app
        .layer(axum::middleware::from_fn_with_state(
            rl_state,
            rate_limit_middleware::global_rate_limit,
        ))
        .layer(axum::middleware::from_fn(common_tracing::request_id_layer))
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );

    // ── 8. Bind listener ──────────────────────────────────────────────────────
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", cfg.app_port))
        .await
        .expect("gagal bind port");

    tracing::info!(port = cfg.app_port, "rejki-app listening");

    // ── 9. Graceful shutdown — drain 30 detik setelah SIGTERM ────────────────
    let shutdown = async {
        tokio::signal::ctrl_c().await.ok();

        #[cfg(unix)]
        {
            use tokio::signal::unix::{signal, SignalKind};
            let mut sigterm = signal(SignalKind::terminate()).expect("failed to register SIGTERM");
            tokio::select! {
                _ = sigterm.recv() => {},
                _ = tokio::signal::ctrl_c() => {},
            }
        }

        tracing::info!("shutdown signal received — draining connections (30s)");
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await
        .expect("server error");

    // Pool closes automatically when pool is dropped
    tracing::info!("rejki-app stopped");
}

async fn health_handler() -> (StatusCode, Json<serde_json::Value>) {
    (StatusCode::OK, Json(json!({ "status": "ok" })))
}
