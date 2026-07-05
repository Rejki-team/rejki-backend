pub mod handlers;

pub mod audit_extractor;

use std::sync::Arc;

use axum::{
    extract::DefaultBodyLimit,
    middleware,
    routing::{get, post},
    Router,
};
use common_idempotency::{idempotency_middleware, IdempotencyState, PgIdempotencyStore};
use sqlx::PgPool;

use crate::application::service::{self, AuthService};
use crate::domain::audit_log::AuditLogRepository;
use crate::domain::rate_limit::RateLimiter;
use crate::domain::token::TokenIssuer;
use crate::infrastructure::{
    AuthInProcessClient, JwtService, OtpRateLimiter, PgAuditLogRepository, PgAuthRepository,
};
use auth_service_client::AuthClient;
use common_auth_mw::{require_active_account, require_auth, require_role};
use notification_service_client::NotificationClient;
use storage_service_client::StorageClient;
use user_service_client::UserClient;

#[derive(Clone)]
pub struct AppState {
    pub auth_svc: Arc<AuthService<PgAuthRepository>>,
    pub storage_client: Option<Arc<dyn StorageClient>>,
    pub user_client: Option<Arc<dyn UserClient>>,
    pub notifier: Option<Arc<dyn NotificationClient>>,
    pub audit_log_repo: Option<Arc<dyn AuditLogRepository>>,
}

/// Entry point yang dipanggil oleh rejki-app (Composition Root) dan main.rs standalone.
/// Menerima shared PgPool dan JwtService yang sudah diinisiasi.
pub fn router_with_jwt(
    pool: PgPool,
    jwt: Arc<JwtService>,
    refresh_ttl_secs: i64,
    redis_url: Option<String>,
) -> Router {
    let repo = Arc::new(PgAuthRepository::new(pool));
    let auth_client: Arc<dyn AuthClient> =
        Arc::new(AuthInProcessClient::new(jwt.clone(), repo.clone()));
    router_with_deps(
        jwt,
        repo,
        auth_client,
        None,
        None,
        refresh_ttl_secs,
        redis_url,
    )
}

/// Bangun router auth dari repository yang sudah dibuat (di-share dengan AuthInProcessClient
/// di Composition Root agar status akun konsisten satu sumber). `auth_client` dipakai untuk
/// melindungi endpoint change-password (require_auth). `notifier` (opsional) untuk email OTP.
/// `storage_client` (opsional) untuk bukti penangguhan (Q1/suspend-evidence).
/// `user_client` (opsional) untuk pemusnahan dokumen KYC saat suspend permanen (D4).
pub fn router_with_deps(
    jwt: Arc<JwtService>,
    repo: Arc<PgAuthRepository>,
    auth_client: Arc<dyn AuthClient>,
    notifier: Option<Arc<dyn NotificationClient>>,
    storage_client: Option<Arc<dyn StorageClient>>,
    refresh_ttl_secs: i64,
    redis_url: Option<String>,
) -> Router {
    router_with_deps_ex(
        jwt,
        repo,
        auth_client,
        notifier,
        storage_client,
        None,
        refresh_ttl_secs,
        redis_url,
    )
}

/// Versi extended — menerima `user_client` untuk pemusnahan dokumen KYC (D4).
#[allow(clippy::too_many_arguments)]
pub fn router_with_deps_ex(
    jwt: Arc<JwtService>,
    repo: Arc<PgAuthRepository>,
    auth_client: Arc<dyn AuthClient>,
    notifier: Option<Arc<dyn NotificationClient>>,
    storage_client: Option<Arc<dyn StorageClient>>,
    user_client: Option<Arc<dyn UserClient>>,
    refresh_ttl_secs: i64,
    redis_url: Option<String>,
) -> Router {
    let token_issuer: Arc<dyn TokenIssuer> = jwt.clone();
    let rate_limiter: Arc<dyn RateLimiter> = Arc::new(OtpRateLimiter::new(redis_url));

    let mut svc = AuthService::new(repo.clone(), token_issuer, refresh_ttl_secs, rate_limiter);
    let notifier_for_state = notifier.clone();
    if let Some(n) = notifier {
        svc = svc.with_notifier(n);
    }

    // Audit log: wire PgAuditLogRepository dari pool yang sama.
    // Fail-open: bila tidak di-wire (mis. standalone test tanpa migration), no-op.
    let audit_log_repo: Arc<dyn AuditLogRepository> =
        Arc::new(PgAuditLogRepository::new(repo.pool()));
    svc = svc.with_audit_log(audit_log_repo.clone());

    let state = AppState {
        auth_svc: Arc::new(svc),
        storage_client,
        user_client,
        notifier: notifier_for_state,
        audit_log_repo: Some(audit_log_repo),
    };
    let pool = repo.pool();
    build_router(state, auth_client, pool)
}

/// Inisiasi JwtService dari env vars, lalu panggil router_with_jwt.
/// Dipakai oleh main.rs standalone binary.
pub fn router(pool: PgPool) -> Router {
    let private_pem = std::fs::read_to_string(
        std::env::var("JWT_PRIVATE_KEY_PATH").unwrap_or_else(|_| "./keys/private.pem".into())
    ).expect("JWT_PRIVATE_KEY_PATH tidak ditemukan — jalankan: openssl genrsa -out keys/private.pem 2048");

    let public_pem = std::fs::read_to_string(
        std::env::var("JWT_PUBLIC_KEY_PATH").unwrap_or_else(|_| "./keys/public.pem".into()),
    )
    .expect("JWT_PUBLIC_KEY_PATH tidak ditemukan");

    let access_ttl = std::env::var("JWT_ACCESS_TTL_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(service::DEFAULT_ACCESS_TTL_SECS);

    let refresh_ttl_secs = std::env::var("JWT_REFRESH_TTL_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(crate::application::service::DEFAULT_REFRESH_TTL_SECS);

    let redis_url = std::env::var("REDIS_URL").ok();

    let jwt = Arc::new(
        JwtService::from_files(&private_pem, &public_pem, access_ttl)
            .expect("gagal inisiasi JwtService"),
    );

    router_with_jwt(pool, jwt, refresh_ttl_secs, redis_url)
}

/// Maksimum ukuran request body untuk endpoint auth: 16 KB.
/// Cukup untuk seluruh DTO (LoginInput, RegisterInput, BulkSuspendInput, dsb).
const MAX_BODY_SIZE: usize = 16 * 1024; // 16 KB

fn build_router(state: AppState, auth_client: Arc<dyn AuthClient>, pool: PgPool) -> Router {
    // Idempotency-Key middleware (W3D-14 D4): pasang per route untuk POST kritis.
    let idempotency_state: IdempotencyState<PgIdempotencyStore> = IdempotencyState {
        store: Arc::new(PgIdempotencyStore::new(pool)),
    };

    // Admin router: require_auth + require_admin + require_active_account (default-deny).
    // require_active_account memastikan hanya admin dengan status Active yang bisa
    // mengakses endpoint admin — suspended admin ditolak dengan 403 ACCOUNT_NOT_ACTIVE.
    // Prefix `/admin` agar konsisten dengan konvensi admin codebase
    // (mis. /auth/admin/login, /users/admin/kyc, /admin/pekerjaan/suspend)
    // dan selaras proposal extend-user-suspension-bulk-purge.
    let admin = Router::new()
        .route(
            "/admin/users/suspend",
            post(handlers::suspend_accounts_bulk),
        )
        .route("/admin/users/{id}/suspend", post(handlers::suspend_account))
        .route(
            "/admin/users/{id}/suspend/evidence",
            post(handlers::request_suspend_evidence),
        )
        .with_state(state.clone())
        .layer(axum::middleware::from_fn_with_state(
            auth_client.clone(),
            require_active_account,
        ))
        .layer(axum::middleware::from_fn_with_state(80u8, require_role))
        .layer(axum::middleware::from_fn_with_state(
            auth_client.clone(),
            require_auth,
        ));

    // Endpoint terautentikasi non-admin (user).
    let protected = Router::new()
        .route("/change-password", post(handlers::change_password))
        .route(
            "/change-password/otp",
            post(handlers::request_change_password_otp),
        )
        .with_state(state.clone())
        .layer(axum::middleware::from_fn_with_state(
            auth_client.clone(),
            require_auth,
        ));

    // D4: Idempotency-Key middleware untuk endpoint POST kritis.
    // register dan login: cegah double-submit akibat retry/network issue.
    let idempotent_routes = Router::new()
        .route("/register", post(handlers::register))
        .route("/login", post(handlers::login))
        .with_state(state.clone())
        .route_layer(middleware::from_fn_with_state(
            idempotency_state.clone(),
            idempotency_middleware::<PgIdempotencyStore>,
        ));

    let public = Router::new()
        .route("/health", get(handlers::health))
        .route("/admin/login", post(handlers::admin_login))
        .route("/verify-otp", post(handlers::verify_otp))
        .route("/resend-otp", post(handlers::resend_otp))
        .route("/refresh", post(handlers::refresh_token))
        .route("/logout", post(handlers::logout))
        .route("/forgot-password", post(handlers::forgot_password))
        .route("/reset-password", post(handlers::reset_password))
        .with_state(state);

    idempotent_routes
        .merge(public)
        .merge(protected)
        .merge(admin)
        .layer(DefaultBodyLimit::max(MAX_BODY_SIZE))
}
