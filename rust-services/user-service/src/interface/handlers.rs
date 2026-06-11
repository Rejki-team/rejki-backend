use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use auth_service_client::AuthClaims;
use common_errors::{ApiResponse, AppError, ValidatedJson};
use storage_service_client::{FileInfo, StorageClientError};

use super::AppState;
use crate::application::dto::{
    AvatarRequest, DocumentRequest, KycPersonalDataInput, KycSubmissionResponse,
    ReviewInput, UpdateProfileInput, UploadPermission, UserProfileResponse,
};

pub async fn get_me(
    State(s): State<AppState>,
    claims: axum::Extension<AuthClaims>,
) -> Result<Json<ApiResponse<UserProfileResponse>>, AppError> {
    let profile = s
        .user_svc
        .get_profile_by_auth_id(claims.user_id)
        .await
        .map_err(|e| AppError::NotFound(e.to_string()))?;
    Ok(Json(ApiResponse::ok(profile)))
}

pub async fn get_by_id(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<UserProfileResponse>>, AppError> {
    let profile = s
        .user_svc
        .get_profile(id)
        .await
        .map_err(|e| AppError::NotFound(e.to_string()))?;
    Ok(Json(ApiResponse::ok(profile)))
}

pub async fn update_me(
    State(s): State<AppState>,
    claims: axum::Extension<AuthClaims>,
    ValidatedJson(body): ValidatedJson<UpdateProfileInput>,
) -> Result<Json<ApiResponse<UserProfileResponse>>, AppError> {
    let profile = s
        .user_svc
        .update_profile(claims.user_id, body)
        .await
        .map_err(AppError::Internal)?;
    Ok(Json(ApiResponse::ok(profile)))
}

/// Health check service (tanpa auth).
pub async fn health() -> (StatusCode, Json<serde_json::Value>) {
    (StatusCode::OK, Json(serde_json::json!({ "status": "ok" })))
}

// ── Avatar (US-09) ──────────────────────────────────────────────────────────

pub async fn request_avatar_upload(
    State(s): State<AppState>,
    claims: axum::Extension<AuthClaims>,
    ValidatedJson(body): ValidatedJson<AvatarRequest>,
) -> Result<Json<ApiResponse<UploadPermission>>, AppError> {
    // Presigned URL via StorageClient (domain storage-service). StorageService
    // memvalidasi MIME & ukuran, lalu membuat object_key sendiri.
    let perm = s
        .storage_client
        .request_upload(
            "avatar",
            claims.user_id,
            FileInfo { mime: body.mime, size_bytes: body.size_bytes },
        )
        .await
        .map_err(map_storage_err)?;

    // Simpan object_key ke profil agar avatar tertaut walau upload belum selesai
    // (klien meng-upload langsung ke storage memakai presigned_url).
    s.user_svc
        .update_avatar(claims.user_id, &perm.object_key)
        .await
        .map_err(AppError::Internal)?;

    Ok(Json(ApiResponse::ok(UploadPermission {
        presigned_url: perm.presigned_url,
        object_key:    perm.object_key,
    })))
}

// ── KYC data diri (US-04) ────────────────────────────────────────────────────

/// Kirim data diri KYC lengkap.
pub async fn submit_kyc(
    State(s): State<AppState>,
    claims: axum::Extension<AuthClaims>,
    ValidatedJson(body): ValidatedJson<KycPersonalDataInput>,
) -> Result<(StatusCode, Json<ApiResponse<KycSubmissionResponse>>), AppError> {
    let submission = s
        .user_svc
        .submit_kyc(claims.user_id, body)
        .await
        .map_err(|e| AppError::Validation(e.to_string()))?;
    Ok((StatusCode::CREATED, Json(ApiResponse::ok(submission))))
}

/// Lihat status KYC sendiri.
pub async fn get_kyc_status(
    State(s): State<AppState>,
    claims: axum::Extension<AuthClaims>,
) -> Result<Json<ApiResponse<Option<KycSubmissionResponse>>>, AppError> {
    let status = s
        .user_svc
        .get_kyc_status(claims.user_id)
        .await
        .map_err(AppError::Internal)?;
    Ok(Json(ApiResponse::ok(status)))
}

// ── Dokumen KYC (US-04) ─────────────────────────────────────────────────────

/// Minta presigned URL untuk upload dokumen KYC (KTP / swafoto).
pub async fn request_document_upload(
    State(s): State<AppState>,
    claims: axum::Extension<AuthClaims>,
    ValidatedJson(body): ValidatedJson<DocumentRequest>,
) -> Result<Json<ApiResponse<UploadPermission>>, AppError> {
    if !["ktp", "selfie"].contains(&body.kind.as_str()) {
        return Err(AppError::Validation("jenis dokumen harus 'ktp' atau 'selfie'".into()));
    }
    // Presigned URL via StorageClient domain — kategori = jenis dokumen (ktp/selfie).
    let perm = s
        .storage_client
        .request_upload(
            &body.kind,
            claims.user_id,
            FileInfo { mime: body.mime, size_bytes: body.size_bytes },
        )
        .await
        .map_err(map_storage_err)?;

    Ok(Json(ApiResponse::ok(UploadPermission {
        presigned_url: perm.presigned_url,
        object_key:    perm.object_key,
    })))
}

// ── Review admin (placeholder RBAC — menyusul) ───────────────────────────────

pub async fn review_kyc(
    State(s): State<AppState>,
    claims: axum::Extension<AuthClaims>,
    Path(submission_id): Path<Uuid>,
    ValidatedJson(body): ValidatedJson<ReviewInput>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), AppError> {
    let note = body.review_note.as_deref();
    // review_kyc sekarang mencari submission→profile→auth_id dan memanggil AuthClient
    s.user_svc
        .review_kyc(submission_id, claims.user_id, body.approved, note)
        .await
        .map_err(AppError::Internal)?;

    Ok((StatusCode::OK, Json(ApiResponse::ok(serde_json::json!(null)))))
}

// ── Pemetaan error StorageClient → AppError ──────────────────────────────────

fn map_storage_err(e: StorageClientError) -> AppError {
    match e {
        StorageClientError::FileTooLarge => {
            AppError::Validation("ukuran berkas melebihi batas".into())
        }
        StorageClientError::InvalidMime => {
            AppError::Validation("tipe berkas tidak didukung".into())
        }
        StorageClientError::Unavailable => {
            AppError::Internal(anyhow::anyhow!("storage service tidak tersedia"))
        }
    }
}
