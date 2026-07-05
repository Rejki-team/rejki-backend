use std::sync::Arc;

use axum::{http::StatusCode, response::Json, routing::get, Router};
use common_config::AppConfig;
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
    // ── 1. Load semua konfigurasi terpusat ────────────────────────────────────
    let cfg = AppConfig::load();
    let is_prod = cfg.is_production();

    // ── 2. Init tracing & OpenTelemetry ──────────────────────────────────────
    #[cfg(feature = "otel")]
    let _otel_handle: Option<common_tracing::OtelHandle> =
        if cfg.otel.exporter_otlp_endpoint.is_some() {
            match common_tracing::init_otel(&cfg.otel, is_prod).await {
                Some(handle) => {
                    tracing::info!("OpenTelemetry mode: spans → Grafana Cloud");
                    Some(handle)
                }
                None => {
                    tracing::warn!(
                        "OTEL endpoint diset tapi init_otel gagal — fallback ke standard tracing"
                    );
                    common_tracing::init_tracing(&cfg.otel, is_prod);
                    None
                }
            }
        } else {
            common_tracing::init_tracing(&cfg.otel, is_prod);
            None
        };
    #[cfg(not(feature = "otel"))]
    {
        let _ = &cfg.otel;
        common_tracing::init_tracing(&cfg.otel, is_prod);
    }

    tracing::info!(
        service = %cfg.otel.service_name,
        env     = ?cfg.app_env,
        "starting rejki-app"
    );

    // ── 3. Database pool (shared — schema terisolasi per service) ──────────────
    let pool = sqlx::PgPool::connect(&cfg.database.url)
        .await
        .expect("gagal connect ke PostgreSQL");

    tracing::info!("database connected");

    // ── 4. JwtService ─────────────────────────────────────────────────────────
    let (private_pem, public_pem) = cfg.jwt.load_keys().expect(
        "JWT key files tidak terbaca — generate: openssl genrsa -out keys/private.pem 2048",
    );

    let jwt = Arc::new(
        auth_service::JwtService::from_files(&private_pem, &public_pem, cfg.jwt.access_ttl_secs)
            .expect("gagal inisiasi JwtService"),
    );

    // ── 5. AuthClient — in-process implementation ──────────────────────────────
    let auth_repo = Arc::new(auth_service::PgAuthRepository::new(pool.clone()));
    let auth_client: Arc<dyn auth_service::AuthClient> = Arc::new(
        auth_service::AuthInProcessClient::new(jwt.clone(), auth_repo.clone()),
    );

    let notifier: Option<Arc<dyn notification_service::NotificationClient>> = cfg
        .redis
        .url
        .as_ref()
        .and_then(|u| notification_service::NotificationPublisher::new(u).ok())
        .map(|p| {
            let p: Arc<dyn notification_service::NotificationClient> = Arc::new(p);
            p
        })
        .or_else(|| {
            tracing::warn!("REDIS_URL tidak diset — email OTP tidak akan dikirim");
            None
        });

    // RegionClient — implementasi in-process untuk user-service (validasi wilayah KYC).
    let region_client: Arc<dyn region_service::RegionClient> = {
        let repo = Arc::new(region_service::PgRegionRepository::new(pool.clone()));
        let svc = Arc::new(region_service::RegionService::new(repo));
        Arc::new(region_service::RegionInProcessClient::new(svc))
    };

    // StorageClient — in-process (MinIO/S3 via presigned URL). MinioStorage di-init
    // dari config; bila config kosong, request_upload mengembalikan Unavailable.
    let storage_client: Arc<dyn storage_service::StorageClient> =
        Arc::new(storage_service::StorageInProcessClient::new().await);

    // UserClient — in-process untuk suspend permanen → purge dokumen KYC (D4).
    let user_client: Arc<dyn user_service::UserClient> = {
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

    // ── 6. Rate limiter (shared — Redis Lua atomik, fail-open) ────────────────
    let rate_limiter: Option<Arc<dyn common_rate_limit::RateLimiter>> =
        if cfg.redis.url.is_some() || cfg!(test) {
            Some(Arc::new(common_rate_limit::OtpRateLimiter::new(
                cfg.redis.url.clone(),
            )))
        } else {
            tracing::warn!("REDIS_URL tidak diset — rate limiter non-aktif");
            None
        };

    // ── 7. Build router ────────────────────────────────────────────────────────
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
                cfg.jwt.refresh_ttl_secs,
                cfg.redis.url.clone(),
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
        )
        .nest(
            "/insights",
            insights_service::router(pool.clone(), auth_client.clone()),
        );

    let mut app = Router::new()
        .route("/health", get(health_handler))
        .nest("/api/v1", api_v1);

    // ── Swagger UI — HANYA development ────────────────────────────────────────
    if cfg.is_development() {
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

    // ── CORS — environment-aware ──────────────────────────────────────────────
    let cors = if is_prod {
        let origins: Vec<_> = cfg
            .cors
            .allowed_origins
            .iter()
            .map(|o| {
                o.parse::<axum::http::HeaderValue>()
                    .expect("CORS_ALLOWED_ORIGINS tidak valid — pastikan format URL benar")
            })
            .collect();
        CorsLayer::new()
            .allow_origin(origins)
            .allow_credentials(true)
            .allow_methods([
                axum::http::Method::GET,
                axum::http::Method::POST,
                axum::http::Method::PATCH,
                axum::http::Method::DELETE,
                axum::http::Method::OPTIONS,
            ])
            .allow_headers([
                axum::http::header::AUTHORIZATION,
                axum::http::header::CONTENT_TYPE,
                axum::http::header::ACCEPT,
            ])
    } else {
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
    };

    let app = app
        .layer(axum::middleware::from_fn_with_state(
            rl_state,
            rate_limit_middleware::global_rate_limit,
        ))
        .layer(axum::middleware::from_fn(common_tracing::request_id_layer))
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .layer(cors);

    // ── 8. Bind listener ───────────────────────────────────────────────────────
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", cfg.app_port))
        .await
        .expect("gagal bind port");

    tracing::info!(port = cfg.app_port, "rejki-app listening");

    // ── 9. Graceful shutdown — drain 30 detik setelah SIGTERM ─────────────────
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

        // Shutdown OpenTelemetry tracer (flush spans)
        #[cfg(feature = "otel")]
        common_tracing::shutdown_otel(_otel_handle).await;
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await
        .expect("server error");

    tracing::info!("rejki-app stopped");
}

async fn health_handler() -> (StatusCode, Json<serde_json::Value>) {
    (StatusCode::OK, Json(json!({ "status": "ok" })))
}
