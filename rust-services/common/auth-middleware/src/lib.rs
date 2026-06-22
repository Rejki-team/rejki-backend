use std::sync::Arc;

use auth_service_client::{AuthClient, AuthClientError};
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use common_errors::AppError;

/// Axum middleware function — gunakan dengan `axum::middleware::from_fn_with_state`.
///
/// Cara pakai di router yang butuh auth:
/// ```ignore
/// let auth_client: Arc<dyn AuthClient> = Arc::new(/* impl konkret */);
/// Router::new()
///     .route("/protected", get(handler))
///     .layer(axum::middleware::from_fn_with_state(auth_client, require_auth))
/// ```
///
/// Handler yang butuh identity user:
/// ```ignore
/// async fn handler(Extension(claims): Extension<AuthClaims>) -> impl IntoResponse { ... }
/// ```
pub async fn require_auth(
    State(auth): State<Arc<dyn AuthClient>>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let token = extract_bearer(req.headers()).ok_or(AppError::Unauthorized)?;

    let claims = auth.validate_token(token).await.map_err(|e| match e {
        AuthClientError::InvalidToken => AppError::Unauthorized,
        AuthClientError::Unavailable => {
            tracing::error!("auth service unavailable during token validation");
            AppError::Unauthorized
        }
        // Tidak relevan untuk validate_token, tetapi wajib exhaustive.
        AuthClientError::NotFound | AuthClientError::InvalidTransition => AppError::Unauthorized,
    })?;

    req.extensions_mut().insert(claims);
    Ok(next.run(req).await)
}

/// Gating fitur: hanya akun berstatus `active` yang boleh lanjut.
///
/// Dipasang SETELAH `require_auth` (yang meng-inject `AuthClaims`). Membaca status
/// dari klaim JWT untuk jalur cepat; bila status di klaim tidak ada (token lama) atau
/// menunjukkan non-active, lakukan pemeriksaan kesegaran ke `AuthClient.get_account_status`
/// sehingga perubahan status (mis. suspend) langsung ditegakkan walau token masih berlaku.
pub async fn require_active_account(
    State(auth): State<Arc<dyn AuthClient>>,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let claims = req
        .extensions()
        .get::<AuthClaims>()
        .cloned()
        .ok_or(AppError::Unauthorized)?;

    // Jalur cepat: status di klaim sudah active → lolos.
    if matches!(claims.status, Some(s) if s.can_use_features()) {
        return Ok(next.run(req).await);
    }

    // Kesegaran: cek status terkini ke sumber kebenaran (auth-service).
    let current = auth
        .get_account_status(claims.user_id)
        .await
        .map_err(|_| AppError::AccountNotActive)?;

    if current.can_use_features() {
        Ok(next.run(req).await)
    } else {
        Err(AppError::AccountNotActive)
    }
}

fn extract_bearer(headers: &axum::http::HeaderMap) -> Option<&str> {
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
}

/// Axum middleware function — otorisasi berbasis role hierarchy.
/// Gunakan dengan `axum::middleware::from_fn_with_state(min_rank, require_role)`.
///
/// Harus dipasang SETELAH `require_auth` (yang sudah meng-inject `AuthClaims`).
/// Menolak `403 INSUFFICIENT_ROLE` bila `AuthClaims.role.rank() < min_rank`.
///
/// # Contoh
/// ```ignore
/// .layer(axum::middleware::from_fn_with_state(80u8, require_role)) // admin iklan ke atas
/// .layer(axum::middleware::from_fn_with_state(60u8, require_role)) // moderator ke atas
/// .layer(axum::middleware::from_fn_with_state(90u8, require_role)) // executive ke atas
/// ```
pub async fn require_role(
    State(min_rank): State<u8>,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let claims = req
        .extensions()
        .get::<AuthClaims>()
        .cloned()
        .ok_or(AppError::Unauthorized)?;

    match claims.role {
        Some(ref role) if role.rank() >= min_rank => Ok(next.run(req).await),
        _ => Err(AppError::InsufficientRole),
    }
}

/// Axum middleware — otorisasi ketat untuk operator admin saja.
/// Hanya SuperAdmin/AdminIklan/AdminUser yang lolos (Executive rank 90 ditolak).
///
/// Gunakan dengan `axum::middleware::from_fn_with_state(auth_client, require_admin_operator)`.
/// Harus dipasang SETELAH `require_auth` (yang sudah meng-inject `AuthClaims`).
pub async fn require_admin_operator(
    State(_auth): State<Arc<dyn AuthClient>>,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let claims = req
        .extensions()
        .get::<AuthClaims>()
        .cloned()
        .ok_or(AppError::Unauthorized)?;

    match claims.role {
        Some(ref role) if role.is_admin_operator() => Ok(next.run(req).await),
        _ => Err(AppError::InsufficientRole),
    }
}

/// Helper untuk extract `AuthClaims` dari Extension dalam handler.
/// Re-export supaya service tidak perlu import auth-service-client langsung.
pub use auth_service_client::AuthClaims;
