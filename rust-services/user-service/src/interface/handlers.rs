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
    AvatarRequest, CommitDocumentInput, DocumentRequest, KycPersonalDataInput,
    KycSubmissionResponse, ReviewInput, UpdateProfileInput, UploadPermission, UserProfileResponse,
};

pub async fn get_me(
    State(s): State<AppState>,
    claims: axum::Extension<AuthClaims>,
) -> Result<Json<ApiResponse<UserProfileResponse>>, AppError> {
    let mut profile = s
        .user_svc
        .get_profile_by_auth_id(claims.user_id)
        .await
        .map_err(|e| AppError::NotFound(e.to_string()))?;
    // Sertakan role dari klaim token untuk dashboard header (FR-ADM-AUTH-04).
    profile.role = claims.role.as_ref().map(|r| r.as_str().to_owned());
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
    let profile_id = s
        .user_svc
        .resolve_profile_id(claims.user_id)
        .await
        .map_err(|e| AppError::NotFound(e.to_string()))?;
    let profile = s
        .user_svc
        .update_profile(profile_id, body)
        .await
        .map_err(AppError::Internal)?;
    Ok(Json(ApiResponse::ok(profile)))
}

pub async fn health() -> (StatusCode, Json<serde_json::Value>) {
    (StatusCode::OK, Json(serde_json::json!({ "status": "ok" })))
}

// ── Avatar (US-09) ──────────────────────────────────────────────────────────

pub async fn request_avatar_upload(
    State(s): State<AppState>,
    claims: axum::Extension<AuthClaims>,
    ValidatedJson(body): ValidatedJson<AvatarRequest>,
) -> Result<Json<ApiResponse<UploadPermission>>, AppError> {
    let profile_id = s
        .user_svc
        .resolve_profile_id(claims.user_id)
        .await
        .map_err(|e| AppError::NotFound(e.to_string()))?;

    let perm = s
        .storage_client
        .request_upload(
            "avatar",
            claims.user_id,
            FileInfo {
                mime: body.mime,
                size_bytes: body.size_bytes,
            },
        )
        .await
        .map_err(map_storage_err)?;

    s.user_svc
        .update_avatar(profile_id, &perm.object_key)
        .await
        .map_err(AppError::Internal)?;

    Ok(Json(ApiResponse::ok(UploadPermission {
        presigned_url: perm.presigned_url,
        object_key: perm.object_key,
    })))
}

// ── KYC data diri (US-04) ────────────────────────────────────────────────────

pub async fn submit_kyc(
    State(s): State<AppState>,
    claims: axum::Extension<AuthClaims>,
    ValidatedJson(body): ValidatedJson<KycPersonalDataInput>,
) -> Result<(StatusCode, Json<ApiResponse<KycSubmissionResponse>>), AppError> {
    let email = claims.email.as_str();
    let submission = s
        .user_svc
        .submit_kyc(claims.user_id, Some(email), body)
        .await
        .map_err(|e| AppError::Validation(e.to_string()))?;
    Ok((StatusCode::CREATED, Json(ApiResponse::ok(submission))))
}

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

pub async fn request_document_upload(
    State(s): State<AppState>,
    claims: axum::Extension<AuthClaims>,
    ValidatedJson(body): ValidatedJson<DocumentRequest>,
) -> Result<Json<ApiResponse<UploadPermission>>, AppError> {
    if !["ktp", "selfie"].contains(&body.kind.as_str()) {
        return Err(AppError::Validation(
            "jenis dokumen harus 'ktp' atau 'selfie'".into(),
        ));
    }
    let perm = s
        .storage_client
        .request_upload(
            &body.kind,
            claims.user_id,
            FileInfo {
                mime: body.mime,
                size_bytes: body.size_bytes,
            },
        )
        .await
        .map_err(map_storage_err)?;

    // Audit: upload_issued tercatat (Q2).
    let _ = s
        .user_svc
        .log_upload_issued(claims.user_id, &perm.object_key, None)
        .await;

    Ok(Json(ApiResponse::ok(UploadPermission {
        presigned_url: perm.presigned_url,
        object_key: perm.object_key,
    })))
}

/// Commit dokumen: klien mengirim header untuk verifikasi magic bytes.
/// Server menyimpan object_key ke kyc_submission.
pub async fn commit_document(
    State(s): State<AppState>,
    claims: axum::Extension<AuthClaims>,
    ValidatedJson(body): ValidatedJson<CommitDocumentInput>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), AppError> {
    s.user_svc
        .commit_document(claims.user_id, None, body)
        .await
        .map_err(|e| AppError::Validation(e.to_string()))?;
    Ok((
        StatusCode::OK,
        Json(ApiResponse::ok(serde_json::json!(null))),
    ))
}

/// Dapatkan presigned URL baca sementara untuk dokumen KYC milik sendiri.
/// Hanya pemilik; akses admin = TODO RBAC.
pub async fn get_document_url(
    State(s): State<AppState>,
    claims: axum::Extension<AuthClaims>,
    Path(kind): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    if !["ktp", "selfie"].contains(&kind.as_str()) {
        return Err(AppError::Validation(
            "jenis dokumen harus 'ktp' atau 'selfie'".into(),
        ));
    }
    let url = s
        .user_svc
        .get_document_url(claims.user_id, &kind, None)
        .await
        .map_err(AppError::Internal)?;

    match url {
        Some(u) => Ok(Json(ApiResponse::ok(serde_json::json!({ "url": u })))),
        None => Err(AppError::NotFound("dokumen belum diupload".into())),
    }
}

// ── Review admin (placeholder RBAC — menyusul) ───────────────────────────────

pub async fn review_kyc(
    State(s): State<AppState>,
    claims: axum::Extension<AuthClaims>,
    Path(submission_id): Path<Uuid>,
    ValidatedJson(body): ValidatedJson<ReviewInput>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), AppError> {
    let note = body.review_note.as_deref();
    s.user_svc
        .review_kyc(submission_id, claims.user_id, body.approved, note)
        .await
        .map_err(AppError::Internal)?;

    Ok((
        StatusCode::OK,
        Json(ApiResponse::ok(serde_json::json!(null))),
    ))
}

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
