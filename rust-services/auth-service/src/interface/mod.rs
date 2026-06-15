pub mod handlers;

use std::env;
use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use sqlx::PgPool;

use crate::application::service::{self, AuthService};
use crate::infrastructure::{AuthInProcessClient, JwtService, PgAuthRepository};
use auth_service_client::AuthClient;
use common_auth_mw::{require_admin, require_auth};
use notification_service_client::NotificationClient;
use storage_service_client::StorageClient;

#[derive(Clone)]
pub struct AppState {
    pub auth_svc: Arc<AuthService<PgAuthRepository>>,
    pub storage_client: Option<Arc<dyn StorageClient>>,
}

/// Entry point yang dipanggil oleh rejki-app (Composition Root) dan main.rs standalone.
/// Menerima shared PgPool dan JwtService yang sudah diinisiasi.
pub fn router_with_jwt(pool: PgPool, jwt: Arc<JwtService>) -> Router {
    let repo = Arc::new(PgAuthRepository::new(pool));
    let auth_client: Arc<dyn AuthClient> =
        Arc::new(AuthInProcessClient::new(jwt.clone(), repo.clone()));
    router_with_deps(jwt, repo, auth_client, None, None)
}

/// Bangun router auth dari repository yang sudah dibuat (di-share dengan AuthInProcessClient
/// di Composition Root agar status akun konsisten satu sumber). `auth_client` dipakai untuk
/// melindungi endpoint change-password (require_auth). `notifier` (opsional) untuk email OTP.
/// `storage_client` (opsional) untuk bukti penangguhan (Q1/suspend-evidence).
pub fn router_with_deps(
    jwt: Arc<JwtService>,
    repo: Arc<PgAuthRepository>,
    auth_client: Arc<dyn AuthClient>,
    notifier: Option<Arc<dyn NotificationClient>>,
    storage_client: Option<Arc<dyn StorageClient>>,
) -> Router {
    let refresh_ttl = env::var("JWT_REFRESH_TTL_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(crate::application::service::DEFAULT_REFRESH_TTL_SECS);

    let mut svc = AuthService::new(repo, jwt, refresh_ttl);
    if let Some(n) = notifier {
        svc = svc.with_notifier(n);
    }

    let state = AppState {
        auth_svc: Arc::new(svc),
        storage_client,
    };

    build_router(state, auth_client)
}

/// Inisiasi JwtService dari env vars, lalu panggil router_with_jwt.
/// Dipakai oleh main.rs standalone binary.
pub fn router(pool: PgPool) -> Router {
    let private_pem = std::fs::read_to_string(
        env::var("JWT_PRIVATE_KEY_PATH").unwrap_or_else(|_| "./keys/private.pem".into())
    ).expect("JWT_PRIVATE_KEY_PATH tidak ditemukan — jalankan: openssl genrsa -out keys/private.pem 2048");

    let public_pem = std::fs::read_to_string(
        env::var("JWT_PUBLIC_KEY_PATH").unwrap_or_else(|_| "./keys/public.pem".into()),
    )
    .expect("JWT_PUBLIC_KEY_PATH tidak ditemukan");

    let access_ttl = env::var("JWT_ACCESS_TTL_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(service::DEFAULT_ACCESS_TTL_SECS);

    let jwt = Arc::new(
        JwtService::from_files(&private_pem, &public_pem, access_ttl)
            .expect("gagal inisiasi JwtService"),
    );

    router_with_jwt(pool, jwt)
}

fn build_router(state: AppState, auth_client: Arc<dyn AuthClient>) -> Router {
    // Admin router: require_auth + require_admin (default-deny).
    let admin = Router::new()
        .route("/users/{id}/suspend", post(handlers::suspend_account))
        .route(
            "/users/{id}/suspend/evidence",
            post(handlers::request_suspend_evidence),
        )
        .with_state(state.clone())
        .layer(axum::middleware::from_fn(require_admin))
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

    let public = Router::new()
        .route("/health", get(handlers::health))
        .route("/register", post(handlers::register))
        .route("/login", post(handlers::login))
        .route("/admin/login", post(handlers::admin_login))
        .route("/verify-otp", post(handlers::verify_otp))
        .route("/resend-otp", post(handlers::resend_otp))
        .route("/refresh", post(handlers::refresh_token))
        .route("/logout", post(handlers::logout))
        .route("/forgot-password", post(handlers::forgot_password))
        .route("/reset-password", post(handlers::reset_password))
        .with_state(state);

    public.merge(protected).merge(admin)
}
