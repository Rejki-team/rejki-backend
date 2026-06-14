use axum::{extract::State, http::StatusCode, response::Response, Json};
use serde::Deserialize;

use crate::application::dto::{
    AdminLoginInput, LoginInput, RegisterInput, ResendOtpInput, VerifyOtpInput,
};
use crate::application::service::storage_category;
use common_errors::{created_response, ApiResponse, AppError, ValidatedJson};

use super::AppState;

/// Health check auth-service (tanpa auth). Mengikuti pola health global rejki-app.
pub async fn health() -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    (
        StatusCode::OK,
        Json(ApiResponse::ok(serde_json::json!({ "status": "ok" }))),
    )
}

pub async fn register(
    State(s): State<AppState>,
    ValidatedJson(body): ValidatedJson<RegisterInput>,
) -> Result<Response, AppError> {
    // Anti-enumeration: register selalu sukses generik untuk input valid; kegagalan
    // di sini berarti galat internal (DB/enkripsi), bukan "email sudah terdaftar".
    s.auth_svc
        .register(body)
        .await
        .map_err(AppError::Internal)?;
    Ok(created_response(
        ApiResponse::ok(serde_json::json!(null)),
        "/api/v1/auth/me",
    ))
}

pub async fn login(
    State(s): State<AppState>,
    ValidatedJson(body): ValidatedJson<LoginInput>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), AppError> {
    let tokens = s
        .auth_svc
        .login(body)
        .await
        .map_err(|e| AppError::Validation(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(ApiResponse::ok(serde_json::json!({
            "access_token":  tokens.access_token,
            "refresh_token": tokens.refresh_token,
            "token_type":    tokens.token_type,
            "expires_in":    tokens.expires_in,
        }))),
    ))
}

/// POST /api/v1/auth/admin/login — login admin email+password tanpa OTP.
/// Anti-enumeration: semua kegagalan dibalut AppError::Unauthorized (seragam).
pub async fn admin_login(
    State(s): State<AppState>,
    ValidatedJson(body): ValidatedJson<AdminLoginInput>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), AppError> {
    let tokens = s
        .auth_svc
        .admin_login(body)
        .await
        .map_err(|_| AppError::Unauthorized)?;

    Ok((
        StatusCode::OK,
        Json(ApiResponse::ok(serde_json::json!({
            "access_token":  tokens.access_token,
            "refresh_token": tokens.refresh_token,
            "token_type":    tokens.token_type,
            "expires_in":    tokens.expires_in,
        }))),
    ))
}

pub async fn verify_otp(
    State(s): State<AppState>,
    ValidatedJson(body): ValidatedJson<VerifyOtpInput>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), AppError> {
    s.auth_svc
        .verify_otp(body)
        .await
        .map_err(|e| AppError::Validation(e.to_string()))?;
    Ok((
        StatusCode::OK,
        Json(ApiResponse::ok(serde_json::json!(null))),
    ))
}

pub async fn resend_otp(
    State(s): State<AppState>,
    ValidatedJson(body): ValidatedJson<ResendOtpInput>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), AppError> {
    s.auth_svc
        .resend_otp(body)
        .await
        .map_err(|e| AppError::Validation(e.to_string()))?;
    Ok((
        StatusCode::OK,
        Json(ApiResponse::ok(serde_json::json!(null))),
    ))
}

#[derive(Deserialize)]
pub struct RefreshBody {
    pub refresh_token: String,
}

pub async fn refresh_token(
    State(s): State<AppState>,
    Json(body): Json<RefreshBody>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), AppError> {
    let input = crate::application::dto::RefreshInput {
        refresh_token: body.refresh_token,
    };
    let tokens = s
        .auth_svc
        .refresh(input)
        .await
        .map_err(|_e| AppError::Unauthorized)?;

    Ok((
        StatusCode::OK,
        Json(ApiResponse::ok(serde_json::json!({
            "access_token":  tokens.access_token,
            "refresh_token": tokens.refresh_token,
            "token_type":    tokens.token_type,
            "expires_in":    tokens.expires_in,
        }))),
    ))
}

#[derive(Deserialize)]
pub struct LogoutBody {
    pub refresh_token: String,
}

pub async fn logout(
    State(s): State<AppState>,
    Json(body): Json<LogoutBody>,
) -> Result<StatusCode, AppError> {
    s.auth_svc
        .logout(&body.refresh_token)
        .await
        .map_err(AppError::Internal)?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Password recovery (US-05 / US-06) ─────────────────────────────────────────

use crate::application::dto::{ChangePasswordInput, ForgotPasswordInput, ResetPasswordInput};
use auth_service_client::AuthClaims;

/// US-05: minta reset password. Selalu 200 generik (anti-enumeration).
pub async fn forgot_password(
    State(s): State<AppState>,
    ValidatedJson(body): ValidatedJson<ForgotPasswordInput>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), AppError> {
    s.auth_svc
        .forgot_password(&body.email)
        .await
        .map_err(AppError::Internal)?;
    Ok((
        StatusCode::OK,
        Json(ApiResponse::ok(serde_json::json!(null))),
    ))
}

/// US-05: set password baru dengan OTP reset.
pub async fn reset_password(
    State(s): State<AppState>,
    ValidatedJson(body): ValidatedJson<ResetPasswordInput>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), AppError> {
    s.auth_svc
        .reset_password(body)
        .await
        .map_err(|e| AppError::Validation(e.to_string()))?;
    Ok((
        StatusCode::OK,
        Json(ApiResponse::ok(serde_json::json!(null))),
    ))
}

/// US-06: minta OTP change-password (authed).
pub async fn request_change_password_otp(
    State(s): State<AppState>,
    claims: axum::Extension<AuthClaims>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), AppError> {
    s.auth_svc
        .request_change_password_otp(claims.user_id)
        .await
        .map_err(AppError::Internal)?;
    Ok((
        StatusCode::OK,
        Json(ApiResponse::ok(serde_json::json!(null))),
    ))
}

/// US-06: ubah password (authed) dengan OTP change-password.
pub async fn change_password(
    State(s): State<AppState>,
    claims: axum::Extension<AuthClaims>,
    ValidatedJson(body): ValidatedJson<ChangePasswordInput>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), AppError> {
    s.auth_svc
        .change_password(claims.user_id, body)
        .await
        .map_err(|e| AppError::Validation(e.to_string()))?;
    Ok((
        StatusCode::OK,
        Json(ApiResponse::ok(serde_json::json!(null))),
    ))
}

// ── Suspend akun (US-07, admin) ───────────────────────────────────────────────

use crate::application::dto::{SuspendEvidenceRequest, SuspendInput};
use uuid::Uuid;

/// US-07 / Q1: minta presigned URL untuk mengunggah bukti penangguhan.
/// Bukti (gambar/PDF) wajib disertakan saat suspend, maks 5MB per dokumen.
pub async fn request_suspend_evidence(
    State(s): State<AppState>,
    claims: axum::Extension<AuthClaims>,
    ValidatedJson(body): ValidatedJson<SuspendEvidenceRequest>,
) -> Result<Json<ApiResponse<storage_service_client::UploadPermission>>, AppError> {
    let storage = s
        .storage_client
        .as_ref()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("storage tidak tersedia")))?;
    let perm = storage
        .request_upload(
            storage_category::SUSPENSION_EVIDENCE,
            claims.user_id,
            storage_service_client::FileInfo {
                mime: body.mime,
                size_bytes: body.size_bytes,
            },
        )
        .await
        .map_err(|e| match e {
            storage_service_client::StorageClientError::FileTooLarge => {
                AppError::Validation("ukuran berkas melebihi batas (maks 5MB)".into())
            }
            storage_service_client::StorageClientError::InvalidMime => {
                AppError::Validation("tipe berkas tidak didukung (JPEG/PNG/PDF)".into())
            }
            storage_service_client::StorageClientError::Unavailable => {
                AppError::Internal(anyhow::anyhow!("storage tidak tersedia"))
            }
        })?;
    Ok(Json(ApiResponse::ok(perm)))
}

/// US-07: tangguhkan akun. Endpoint internal — saat ini hanya require_auth;
/// proteksi RBAC admin penuh menyusul di proposal Web Dashboard.
/// `admin_id` diambil dari identitas pemanggil (placeholder hingga RBAC tersedia).
/// evidence_object_key wajib diisi (Q1: bukti penangguhan demi audit & sengketa).
pub async fn suspend_account(
    State(s): State<AppState>,
    claims: axum::Extension<AuthClaims>,
    axum::extract::Path(target_user_id): axum::extract::Path<Uuid>,
    ValidatedJson(body): ValidatedJson<SuspendInput>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), AppError> {
    if !body.permanent && body.expires_at.is_none() {
        return Err(AppError::Validation(
            "expires_at wajib untuk penangguhan sementara".into(),
        ));
    }
    s.auth_svc
        .suspend_account(
            target_user_id,
            body.permanent,
            &body.reason,
            body.expires_at,
            claims.user_id,
            body.evidence_object_key.as_deref(),
        )
        .await
        .map_err(AppError::Internal)?;
    Ok((
        StatusCode::OK,
        Json(ApiResponse::ok(serde_json::json!(null))),
    ))
}
