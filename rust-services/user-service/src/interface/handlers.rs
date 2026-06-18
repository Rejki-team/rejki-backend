use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::Response,
    Json,
};
use uuid::Uuid;

use auth_service_client::AuthClaims;
use common_errors::{ApiResponse, AppError, PaginatedMeta, ValidatedJson};
use storage_service_client::{FileInfo, StorageClientError};

use super::AppState;
use crate::application::dto::{
    AdminKycListQuery, AvatarRequest, CommitDocumentInput, DocumentRequest, KycPersonalDataInput,
    KycSubmissionResponse, ReviewInput, UpdateProfileInput, UploadPermission, UserProfileResponse,
};
use crate::domain::entity::ReviewError;

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
    claims: axum::Extension<AuthClaims>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<UserProfileResponse>>, AppError> {
    // Ownership check: hanya pemilik yang bisa melihat profilnya (C3).
    if claims.user_id != id {
        return Err(AppError::NotFound("profil tidak ditemukan".into()));
    }
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

    // Audit: upload_issued tercatat (Q2) — wajib, error dipropagasi (H3).
    s.user_svc
        .log_upload_issued(claims.user_id, &perm.object_key, None)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("gagal catat audit upload: {e}")))?;

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

// ── Review admin (require_admin) ──────────────────────────────────────────────

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
        .map_err(map_review_err)?;

    Ok((
        StatusCode::OK,
        Json(ApiResponse::ok(serde_json::json!(null))),
    ))
}

// ── Admin: listing, detail, dokumen, CSV (add-user-admin-management) ──────────

/// GET /admin/kyc — daftar pengajuan KYC (search/filter/sort/pagination). NIK ter-mask.
pub async fn admin_list_kyc(
    State(s): State<AppState>,
    Query(q): Query<AdminKycListQuery>,
) -> Result<Json<ApiResponse<Vec<crate::application::dto::AdminKycListItem>>>, AppError> {
    let page = s
        .user_svc
        .admin_list_submissions(q)
        .await
        .map_err(AppError::Internal)?;
    let meta = PaginatedMeta::new(page.page, page.per_page, page.total);
    Ok(Json(ApiResponse::with_meta(page.items, meta)))
}

/// GET /admin/kyc/{id} — detail satu pengajuan (pop-up). NIK ter-mask; IDOR→404.
pub async fn admin_get_kyc(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<crate::application::dto::AdminKycDetail>>, AppError> {
    let detail = s
        .user_svc
        .admin_get_submission(id)
        .await
        .map_err(AppError::Internal)?
        .ok_or_else(|| AppError::NotFound("pengajuan tidak ditemukan".into()))?;
    Ok(Json(ApiResponse::ok(detail)))
}

/// GET /admin/kyc/{id}/documents/{kind} — presigned read URL dokumen (teraudit).
pub async fn admin_get_document(
    State(s): State<AppState>,
    claims: axum::Extension<AuthClaims>,
    Path((id, kind)): Path<(Uuid, String)>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    if !["ktp", "selfie"].contains(&kind.as_str()) {
        return Err(AppError::Validation(
            "jenis dokumen harus 'ktp' atau 'selfie'".into(),
        ));
    }
    // request_id untuk audit yang akurat (layer tracing sudah inject header).
    let request_id = headers.get("x-request-id").and_then(|v| v.to_str().ok());

    let url = s
        .user_svc
        .admin_get_document_url(id, &kind, claims.user_id, request_id)
        .await
        .map_err(AppError::Internal)?;

    match url {
        Some(u) => Ok(Json(ApiResponse::ok(serde_json::json!({ "url": u })))),
        None => Err(AppError::NotFound("dokumen tidak tersedia".into())),
    }
}

/// GET /admin/kyc/export.csv — ekspor daftar pengajuan sesuai filter aktif.
pub async fn admin_export_kyc_csv(
    State(s): State<AppState>,
    Query(q): Query<AdminKycListQuery>,
) -> Result<Response, AppError> {
    let items = s
        .user_svc
        .admin_export_submissions(q)
        .await
        .map_err(AppError::Internal)?;

    let mut csv = String::from(
        "ID,Nama,Pendidikan,Gender,Tanggal_Lahir,Alamat,Provinsi_ID,Kabupaten_ID,Kecamatan_ID,Kelurahan_ID,Negara,Status,Created_At\n",
    );
    for item in &items {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
            item.id,
            escape_csv(item.full_name.as_deref().unwrap_or("")),
            escape_csv(item.education_level.as_deref().unwrap_or("")),
            escape_csv(item.gender.as_deref().unwrap_or("")),
            item.birth_date.map(|d| d.to_string()).unwrap_or_default(),
            escape_csv(item.address_line.as_deref().unwrap_or("")),
            item.province_id.as_deref().unwrap_or(""),
            item.regency_id.as_deref().unwrap_or(""),
            item.district_id.as_deref().unwrap_or(""),
            item.village_id.as_deref().unwrap_or(""),
            escape_csv(&item.country_code),
            item.status,
            item.created_at,
        ));
    }

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
        .header(
            header::CONTENT_DISPOSITION,
            "attachment; filename=kyc_submissions.csv",
        )
        .body(axum::body::Body::from(csv))
        .map_err(|e| AppError::Internal(anyhow::anyhow!("gagal membangun respons CSV: {e}")))
}

/// Petakan galat domain review ke kode HTTP yang tepat (D5).
fn map_review_err(e: ReviewError) -> AppError {
    match e {
        ReviewError::AlreadyReviewed => {
            AppError::Conflict("pengajuan sudah ditinjau dan tidak dapat ditinjau ulang".into())
        }
        ReviewError::NotFound => AppError::NotFound("submission tidak ditemukan".into()),
        ReviewError::Other(err) => AppError::Internal(err),
    }
}

/// Escape sel CSV bila mengandung koma/kutip/newline (RFC 4180).
/// Cegah CSV formula injection (CWE-1236): karakter `= + - @` di awal cell
/// diprefix dengan tab agar tidak dieksekusi oleh Excel/LibreOffice Calc (M3).
fn escape_csv(s: &str) -> String {
    // CWE-1236: periksa karakter formula injection (= + - @) SEBELUM RFC 4180 quoting.
    // Jika quoting dilakukan dulu, sel yang dimulai dengan "=", "+", "−", atau "@"
    // menjadi diapit tanda kutip dan karakter awal menjadi """ → lolos dari deteksi.
    let with_prefix = if s.starts_with(['=', '+', '-', '@']) {
        format!("\t{s}")
    } else {
        s.to_string()
    };
    // RFC 4180 quoting: bungkus dengan double-quote bila mengandung koma, kutip, atau newline
    if with_prefix.contains(',') || with_prefix.contains('"') || with_prefix.contains('\n') {
        format!("\"{}\"", with_prefix.replace('"', "\"\""))
    } else {
        with_prefix
    }
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
