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
        .and_then(|u| notification_service::NotificationPublisher::new(u, pool.clone()).ok())
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

    // GeocodingClient — Nominatim (OpenStreetMap) publik, dipakai user-service + 4 service
    // iklan untuk konversi alamat→koordinat saat create/update (F-1). Shared satu instance
    // (reqwest::Client internal sudah connection-pooled, aman di-share via Arc).
    let geocoding_client: Arc<dyn common_geocoding::GeocodingClient> =
        Arc::new(common_geocoding::NominatimGeocodingClient::new());

    // UserClient — in-process untuk suspend permanen → purge dokumen KYC (D4).
    let user_client: Arc<dyn user_service::UserClient> = {
        let user_repo = Arc::new(user_service::PgUserRepository::new(pool.clone()));
        let user_svc = Arc::new(user_service::UserService::new(
            user_repo,
            auth_client.clone(),
            region_client.clone(),
            Some(storage_client.clone()),
            notifier.clone(),
            Some(geocoding_client.clone()),
        ));
        Arc::new(user_service::UserInProcessClient::new(user_svc))
    };

    // IklanPekerjaClient — in-process, dipakai iklan-pekerjaan-service untuk validasi
    // P1.3 "sudah punya Iklan Pekerja aktif" sebelum melamar (F-3, Kelompok 3 Phase 1).
    let iklan_pekerja_client: Arc<dyn iklan_pekerja_service::IklanPekerjaClient> = Arc::new(
        iklan_pekerja_service::IklanPekerjaInProcessClient::new(Arc::new(
            iklan_pekerja_service::PgIklanPekerjaRepository::new(pool.clone()),
        )),
    );

    // IklanPekerjaanClient — in-process, dipakai rating-service untuk validasi
    // "lamaran sudah Selesai" sebelum menerima rating (F-17, Kelompok 3 Phase 5).
    let iklan_pekerjaan_client: Arc<dyn iklan_pekerjaan_service::IklanPekerjaanClient> = Arc::new(
        iklan_pekerjaan_service::IklanPekerjaanInProcessClient::new(Arc::new(
            iklan_pekerjaan_service::PgIklanPekerjaanRepository::new(pool.clone()),
        )),
    );

    // IklanBarangBekasClient — in-process, dipakai report-service untuk endpoint
    // approve-and-suspend saat target_ad_type=barang_bekas (P9.1, Kelompok 6 Q9).
    let iklan_barang_bekas_client: Arc<dyn iklan_barang_bekas_service::IklanBarangBekasClient> =
        Arc::new(
            iklan_barang_bekas_service::IklanBarangBekasInProcessClient::new(Arc::new(
                iklan_barang_bekas_service::PgIklanBarangBekasRepository::new(pool.clone()),
            )),
        );

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

    // SchedulerClient — producer tugas otomatis Bab 10 (F-32, Kelompok 2 Phase 6).
    // Fail-open: REDIS_URL kosong → schedule() selalu Err(Unconfigured), pemanggil
    // sudah menangani ini sebagai warn! (I/O aman §4.5), tidak menggagalkan create/enroll.
    let scheduler_client = Arc::new(common_scheduler::SchedulerClient::new(
        cfg.redis.url.clone(),
    ));

    // PgChatRepository — dipakai bersama oleh ChatClient in-process (di bawah) &
    // handler scheduler chat (registry di bawah) — instance terpisah dari yang
    // dibangun `chat_service::router()` sendiri, pola sama iklan_pekerja_client/
    // iklan_pekerjaan_client (masing-masing punya repo in-process sendiri).
    let chat_repo = Arc::new(chat_service::PgChatRepository::new(pool.clone()));

    // ChatClient — in-process, dipakai iklan-pekerjaan-service & iklan-barang-bekas-
    // service untuk menjadwalkan auto-end percakapan 2x24 jam setelah "proses pada
    // iklan terkait selesai" (F-19, Kelompok 4 Phase 4).
    let chat_client: Arc<dyn chat_service::ChatClient> = {
        let mut b = chat_service::ChatService::new(chat_repo.clone());
        if let Some(rl) = rate_limiter.clone() {
            b = b.with_rate_limiter(rl);
        }
        b = b.with_scheduler_client(scheduler_client.clone());
        Arc::new(chat_service::ChatInProcessClient::new(Arc::new(b)))
    };

    // Registry + consumer tugas otomatis Bab 10 — hanya jalan bila REDIS_URL diset
    // (fail-open, konsisten dengan rate limiter di atas).
    if let Some(redis_url) = cfg.redis.url.clone() {
        let pelatihan_repo = Arc::new(
            iklan_pelatihan_service::infrastructure::PgIklanPelatihanRepository::new(pool.clone()),
        );
        let mut registry = common_scheduler::SchedulerRegistry::new();
        registry.register(
            iklan_pelatihan_service::application::scheduled_jobs::JOB_CANCEL_UNVERIFIED,
            Arc::new(
                iklan_pelatihan_service::application::scheduled_jobs::CancelUnverifiedHandler {
                    repo: pelatihan_repo.clone(),
                },
            ),
        );
        registry.register(
            iklan_pelatihan_service::application::scheduled_jobs::JOB_MARK_SELESAI,
            Arc::new(
                iklan_pelatihan_service::application::scheduled_jobs::MarkSelesaiHandler {
                    repo: pelatihan_repo.clone(),
                },
            ),
        );
        registry.register(
            iklan_pelatihan_service::application::scheduled_jobs::JOB_CHECK_BADGE_DEADLINE,
            Arc::new(
                iklan_pelatihan_service::application::scheduled_jobs::CheckBadgeDeadlineHandler {
                    repo: pelatihan_repo.clone(),
                    auth_client: auth_client.clone(),
                },
            ),
        );
        registry.register(
            iklan_pelatihan_service::application::scheduled_jobs::JOB_CANCEL_ENROLLMENT_UNPAID,
            Arc::new(iklan_pelatihan_service::application::scheduled_jobs::CancelEnrollmentUnpaidHandler {
                repo: pelatihan_repo,
            }),
        );

        // Chat (F-19, Kelompok 4 Phase 4): auto-end 2x24 jam + retensi 60 hari.
        registry.register(
            chat_service::application::scheduled_jobs::JOB_CHAT_AUTO_END,
            Arc::new(chat_service::application::scheduled_jobs::AutoEndHandler {
                repo: chat_repo.clone(),
            }),
        );
        registry.register(
            chat_service::application::scheduled_jobs::JOB_CHAT_RETENTION_PURGE,
            Arc::new(
                chat_service::application::scheduled_jobs::RetentionPurgeHandler {
                    repo: chat_repo.clone(),
                },
            ),
        );

        match common_scheduler::SchedulerConsumer::new(&redis_url, registry, "rejki-app") {
            Ok(consumer) => {
                tokio::spawn(async move {
                    if let Err(e) = consumer.run().await {
                        tracing::error!(error = ?e, "scheduler consumer berhenti (fatal)");
                    }
                });
                tracing::info!("scheduler tugas otomatis Bab 10 (F-32) aktif");
            }
            Err(e) => {
                tracing::warn!(error = ?e, "gagal membuat scheduler consumer — tugas otomatis Bab 10 non-aktif");
            }
        }
    } else {
        tracing::warn!("REDIS_URL tidak diset — scheduler tugas otomatis Bab 10 non-aktif");
    }

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
                Some(geocoding_client.clone()),
            ),
        )
        .nest(
            "/chat",
            chat_service::router(
                pool.clone(),
                auth_client.clone(),
                rate_limiter.clone(),
                Some(scheduler_client.clone()),
                Some(storage_client.clone()),
                Some(user_client.clone()),
            ),
        )
        .nest(
            "/notif",
            notification_service::router(pool.clone(), auth_client.clone(), rate_limiter.clone()),
        )
        .nest(
            "/pekerjaan",
            iklan_pekerjaan_service::router(iklan_pekerjaan_service::RouterDeps {
                pool: pool.clone(),
                auth_client: auth_client.clone(),
                storage: Some(storage_client.clone()),
                notifier: notifier.clone(),
                rate_limiter: rate_limiter.clone(),
                region_client: Some(region_client.clone()),
                geocoding_client: Some(geocoding_client.clone()),
                iklan_pekerja_client: Some(iklan_pekerja_client.clone()),
                chat_client: Some(chat_client.clone()),
            }),
        )
        .nest(
            "/pekerja",
            iklan_pekerja_service::router(iklan_pekerja_service::RouterDeps {
                pool: pool.clone(),
                auth_client: auth_client.clone(),
                storage: Some(storage_client.clone()),
                notifier: notifier.clone(),
                rate_limiter: rate_limiter.clone(),
                region_client: Some(region_client.clone()),
                user_client: Some(user_client.clone()),
                geocoding_client: Some(geocoding_client.clone()),
            }),
        )
        .nest(
            "/barang",
            iklan_barang_bekas_service::router(iklan_barang_bekas_service::RouterDeps {
                pool: pool.clone(),
                auth_client: auth_client.clone(),
                storage: Some(storage_client.clone()),
                notifier: notifier.clone(),
                rate_limiter: rate_limiter.clone(),
                region_client: Some(region_client.clone()),
                geocoding_client: Some(geocoding_client.clone()),
                user_client: Some(user_client.clone()),
                chat_client: Some(chat_client.clone()),
            }),
        )
        .nest(
            "/pelatihan",
            iklan_pelatihan_service::router(iklan_pelatihan_service::RouterDeps {
                pool: pool.clone(),
                auth_client: auth_client.clone(),
                storage: Some(storage_client.clone()),
                notifier: notifier.clone(),
                rate_limiter: rate_limiter.clone(),
                region_client: Some(region_client.clone()),
                geocoding_client: Some(geocoding_client.clone()),
                scheduler_client: Some(scheduler_client.clone()),
                user_client: Some(user_client.clone()),
            }),
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
            report_service::router(report_service::RouterDeps {
                pool: pool.clone(),
                auth_client: auth_client.clone(),
                storage: Some(storage_client.clone()),
                notifier: notifier.clone(),
                rate_limiter: rate_limiter.clone(),
                user_client: Some(user_client.clone()),
                region_client: Some(region_client.clone()),
                iklan_pekerjaan_client: Some(iklan_pekerjaan_client.clone()),
                iklan_pekerja_client: Some(iklan_pekerja_client.clone()),
                iklan_barang_bekas_client: Some(iklan_barang_bekas_client.clone()),
            }),
        )
        .nest(
            "/insights",
            insights_service::router(pool.clone(), auth_client.clone()),
        )
        .nest(
            "/rating",
            rating_service::router(
                pool.clone(),
                auth_client.clone(),
                Some(iklan_pekerjaan_client.clone()),
            ),
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
