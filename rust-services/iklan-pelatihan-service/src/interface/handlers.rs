use super::AppState;
use crate::application::dto::{
    AdminListQuery, BadgeEvidenceInput, BadgeListQuery, BadgeResponse, CommitBadgeSertifikatInput,
    CommitEnrollBuktiInput, CreateIklanPelatihanInput, EnrollEvidenceInput, EnrollmentListQuery,
    EnrollmentResponse, IklanPelatihanResponse, ListQuery, PelatihanListQuery, ReviewBadgeInput,
    ReviewEnrollmentInput, ReviewPelatihanInput, SuspendEvidenceInput, SuspendInput,
    SuspendResponse, UpdateIklanPelatihanInput, UpdatePelatihanInput,
};
use auth_service_client::AuthClaims;
use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::Response,
    Extension, Json,
};
use common_errors::{created_response, ApiResponse, AppError, ValidatedJson};
use uuid::Uuid;

// ═══════════════════════════════════════════════════════════════════════════
// Public
// ═══════════════════════════════════════════════════════════════════════════

pub async fn health() -> (StatusCode, Json<serde_json::Value>) {
    (StatusCode::OK, Json(serde_json::json!({ "status": "ok" })))
}

pub async fn list(
    State(s): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<ApiResponse<Vec<IklanPelatihanResponse>>>, AppError> {
    Ok(Json(ApiResponse::ok(
        s.svc.list(q).await.map_err(AppError::Internal)?,
    )))
}

pub async fn get_by_id(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<IklanPelatihanResponse>>, AppError> {
    Ok(Json(ApiResponse::ok(
        s.svc
            .get(id)
            .await
            .map_err(|e| AppError::NotFound(e.to_string()))?,
    )))
}

// ── User create (existing POST /) ────────────────────────────────────────

pub async fn create(
    State(s): State<AppState>,
    Extension(claims): Extension<AuthClaims>,
    ValidatedJson(body): ValidatedJson<CreateIklanPelatihanInput>,
) -> Result<Response, AppError> {
    let item = s.svc.create_user(claims.user_id, body).await.map_err(|e| {
        let msg = e.to_string();
        if msg.contains("terlalu banyak permintaan") {
            AppError::RateLimited(msg)
        } else {
            AppError::Internal(e)
        }
    })?;
    let id = item.id;
    Ok(created_response(
        ApiResponse::ok(item),
        &format!("/api/v1/pelatihan/{id}"),
    ))
}

pub async fn delete_iklan(
    State(s): State<AppState>,
    Extension(claims): Extension<AuthClaims>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    if s.svc
        .delete(id, claims.user_id)
        .await
        .map_err(AppError::Internal)?
    {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound("tidak ditemukan".into()))
    }
}

/// PATCH — user-level partial update (only editable fields that the plan maps).
pub async fn update(
    State(s): State<AppState>,
    Extension(claims): Extension<AuthClaims>,
    Path(id): Path<Uuid>,
    ValidatedJson(body): ValidatedJson<UpdatePelatihanInput>,
) -> Result<Json<ApiResponse<IklanPelatihanResponse>>, AppError> {
    let result = s.svc.update(claims.user_id, id, body).await.map_err(|e| {
        let msg = e.to_string();
        if msg.contains("terlalu banyak permintaan") {
            AppError::RateLimited(msg)
        } else if msg.contains("ditangguhkan")
            || msg.contains("tidak dapat diubah pada status saat ini")
        {
            AppError::Forbidden(msg)
        } else if msg.contains("tidak ditemukan") {
            AppError::NotFound(msg)
        } else {
            AppError::Internal(e)
        }
    })?;
    Ok(Json(ApiResponse::ok(result)))
}

// ═══════════════════════════════════════════════════════════════════════════
// Admin: pelatihan management
// ═══════════════════════════════════════════════════════════════════════════

/// Admin create pelatihan (auto-approve → verifikasi_diterima)
pub async fn admin_create_pelatihan(
    State(s): State<AppState>,
    Extension(claims): Extension<AuthClaims>,
    ValidatedJson(body): ValidatedJson<CreateIklanPelatihanInput>,
) -> Result<Response, AppError> {
    let item = s
        .svc
        .create_admin(claims.user_id, body)
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("terlalu banyak permintaan") {
                AppError::RateLimited(msg)
            } else {
                AppError::Internal(e)
            }
        })?;
    let id = item.id;
    Ok(created_response(
        ApiResponse::ok(item),
        &format!("/api/v1/pelatihan/{id}"),
    ))
}

/// Admin list pelatihan (7-stage status)
pub async fn admin_pelatihan_list(
    State(s): State<AppState>,
    Query(q): Query<PelatihanListQuery>,
) -> Result<Json<ApiResponse<Vec<crate::application::dto::AdminIklanPelatihanResponse>>>, AppError>
{
    let (items, total) = s
        .svc
        .admin_pelatihan_list(q)
        .await
        .map_err(AppError::Internal)?;
    let meta = common_errors::PaginatedMeta::new(1, 20, total);
    Ok(Json(ApiResponse::with_meta(items, meta)))
}

/// Admin update pelatihan (own only, admin role)
pub async fn admin_update_pelatihan(
    State(s): State<AppState>,
    Extension(claims): Extension<AuthClaims>,
    Path(id): Path<Uuid>,
    ValidatedJson(body): ValidatedJson<UpdateIklanPelatihanInput>,
) -> Result<Json<ApiResponse<crate::application::dto::AdminIklanPelatihanResponse>>, AppError> {
    let updated = s
        .svc
        .admin_update_pelatihan(claims.user_id, id, body)
        .await
        .map_err(|e| AppError::Forbidden(e.to_string()))?;
    Ok(Json(ApiResponse::ok(updated)))
}

/// Admin cancel pelatihan (soft delete, own only)
pub async fn admin_cancel_pelatihan(
    State(s): State<AppState>,
    Extension(claims): Extension<AuthClaims>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let deleted = s
        .svc
        .admin_cancel_pelatihan(claims.user_id, id)
        .await
        .map_err(|e| AppError::Forbidden(e.to_string()))?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound("tidak ditemukan".into()))
    }
}

/// Admin review pelatihan (approve/reject)
pub async fn admin_review_pelatihan(
    State(s): State<AppState>,
    Extension(claims): Extension<AuthClaims>,
    Path(id): Path<Uuid>,
    Json(body): Json<ReviewPelatihanInput>,
) -> Result<Json<ApiResponse<crate::application::dto::AdminIklanPelatihanResponse>>, AppError> {
    let result = s
        .svc
        .admin_review_pelatihan(
            claims.user_id,
            id,
            body,
            s.notifier.as_deref(),
            s.auth_client.as_deref(),
        )
        .await
        .map_err(|e| AppError::Validation(e.to_string()))?;
    Ok(Json(ApiResponse::ok(result)))
}

/// Export CSV pelatihan
pub async fn admin_pelatihan_export_csv(
    State(s): State<AppState>,
    Query(q): Query<PelatihanListQuery>,
) -> Result<Response, AppError> {
    let items = s
        .svc
        .admin_pelatihan_export_csv(q)
        .await
        .map_err(AppError::Internal)?;
    let mut csv = String::from("ID,Judul,Penyelenggara,Deskripsi,Lokasi,Harga,Tanggal_Mulai,Tanggal_Selesai,Jumlah_Peserta,Status,Pembuat,Moderation_Status,Created_At\n");
    for item in &items {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
            item.id,
            escape_csv(&item.judul),
            escape_csv(&item.penyelenggara),
            escape_csv(&item.deskripsi),
            item.lokasi.as_deref().unwrap_or(""),
            item.harga.map(|v| v.to_string()).unwrap_or_default(),
            item.tanggal_mulai
                .map(|v| v.to_string())
                .unwrap_or_default(),
            item.tanggal_selesai
                .map(|v| v.to_string())
                .unwrap_or_default(),
            item.jumlah_peserta
                .map(|v| v.to_string())
                .unwrap_or_default(),
            item.status,
            item.created_by_role,
            item.moderation_status,
            item.created_at
        ));
    }
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
        .header(
            header::CONTENT_DISPOSITION,
            "attachment; filename=pelatihan.csv",
        )
        .body(axum::body::Body::from(csv))
        .unwrap())
}

// ═══════════════════════════════════════════════════════════════════════════
// Admin: moderation / suspension (existing)
// ═══════════════════════════════════════════════════════════════════════════

pub async fn admin_list(
    State(s): State<AppState>,
    Query(q): Query<AdminListQuery>,
) -> Result<Json<ApiResponse<Vec<crate::application::dto::AdminIklanPelatihanResponse>>>, AppError>
{
    let (items, total) = s.svc.admin_list(q).await.map_err(AppError::Internal)?;
    let meta = common_errors::PaginatedMeta::new(1, 20, total);
    Ok(Json(ApiResponse::with_meta(items, meta)))
}

pub async fn admin_export_csv(
    State(s): State<AppState>,
    Query(q): Query<AdminListQuery>,
) -> Result<Response, AppError> {
    let items = s
        .svc
        .admin_export_csv(q)
        .await
        .map_err(AppError::Internal)?;
    let mut csv = String::from("ID,Judul,Penyelenggara,Deskripsi,Lokasi,Harga,Tanggal_Mulai,Tanggal_Selesai,Status_Moderasi,Status,Pembuat,Created_At\n");
    for item in &items {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{}\n",
            item.id,
            escape_csv(&item.judul),
            escape_csv(&item.penyelenggara),
            escape_csv(&item.deskripsi),
            item.lokasi.as_deref().unwrap_or(""),
            item.harga.map(|v| v.to_string()).unwrap_or_default(),
            item.tanggal_mulai
                .map(|v| v.to_string())
                .unwrap_or_default(),
            item.tanggal_selesai
                .map(|v| v.to_string())
                .unwrap_or_default(),
            item.moderation_status,
            item.status,
            item.created_by_role,
            item.created_at
        ));
    }
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
        .header(
            header::CONTENT_DISPOSITION,
            "attachment; filename=iklan_pelatihan_moderation.csv",
        )
        .body(axum::body::Body::from(csv))
        .unwrap())
}

pub async fn admin_request_evidence(
    State(s): State<AppState>,
    Extension(claims): Extension<AuthClaims>,
    ValidatedJson(body): ValidatedJson<SuspendEvidenceInput>,
) -> Result<Json<ApiResponse<storage_service_client::UploadPermission>>, AppError> {
    let storage = s
        .storage
        .as_ref()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("storage tidak tersedia")))?;
    let perm = s
        .svc
        .request_suspend_evidence(storage.as_ref(), claims.user_id, body)
        .await
        .map_err(AppError::Internal)?;
    Ok(Json(ApiResponse::ok(perm)))
}

pub async fn admin_suspend(
    State(s): State<AppState>,
    Extension(claims): Extension<AuthClaims>,
    ValidatedJson(body): ValidatedJson<SuspendInput>,
) -> Result<Json<ApiResponse<SuspendResponse>>, AppError> {
    let result = s
        .svc
        .suspend(
            claims.user_id,
            body,
            s.notifier.as_deref(),
            s.auth_client.as_deref(),
        )
        .await
        .map_err(AppError::Internal)?;
    Ok(Json(ApiResponse::ok(result)))
}

// ═══════════════════════════════════════════════════════════════════════════
// Enrollment (user endpoints)
// ═══════════════════════════════════════════════════════════════════════════

/// User minta presigned URL bukti transfer
pub async fn request_enroll_evidence(
    State(s): State<AppState>,
    Extension(claims): Extension<AuthClaims>,
    ValidatedJson(body): ValidatedJson<EnrollEvidenceInput>,
) -> Result<Json<ApiResponse<storage_service_client::UploadPermission>>, AppError> {
    let storage = s
        .storage
        .as_ref()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("storage tidak tersedia")))?;
    let perm = s
        .svc
        .request_enroll_evidence(storage.as_ref(), claims.user_id, body)
        .await
        .map_err(AppError::Internal)?;
    Ok(Json(ApiResponse::ok(perm)))
}

/// User mendaftar pelatihan (POST /pelatihan/{id}/enroll)
pub async fn create_enrollment(
    State(s): State<AppState>,
    Extension(claims): Extension<AuthClaims>,
    Path(pelatihan_id): Path<Uuid>,
) -> Result<(StatusCode, Json<ApiResponse<EnrollmentResponse>>), AppError> {
    let enrollment = s
        .svc
        .create_enrollment(pelatihan_id, claims.user_id)
        .await
        .map_err(|e| AppError::NotFound(e.to_string()))?;
    Ok((StatusCode::CREATED, Json(ApiResponse::ok(enrollment))))
}

/// User commit bukti transfer (POST /pelatihan/enrollments/{id}/commit-bukti)
pub async fn commit_enrollment_bukti(
    State(s): State<AppState>,
    Extension(claims): Extension<AuthClaims>,
    Path(id): Path<Uuid>,
    Json(body): Json<CommitEnrollBuktiInput>,
) -> Result<Json<ApiResponse<EnrollmentResponse>>, AppError> {
    let updated = s
        .svc
        .commit_enrollment_bukti(id, claims.user_id, body)
        .await
        .map_err(|e| AppError::Validation(e.to_string()))?;
    Ok(Json(ApiResponse::ok(updated)))
}

// ═══════════════════════════════════════════════════════════════════════════
// Admin: enrollment management
// ═══════════════════════════════════════════════════════════════════════════

pub async fn admin_enrollment_list(
    State(s): State<AppState>,
    Query(q): Query<EnrollmentListQuery>,
) -> Result<Json<ApiResponse<Vec<EnrollmentResponse>>>, AppError> {
    let (items, total) = s
        .svc
        .admin_enrollment_list(q)
        .await
        .map_err(AppError::Internal)?;
    let meta = common_errors::PaginatedMeta::new(1, 20, total);
    Ok(Json(ApiResponse::with_meta(items, meta)))
}

pub async fn admin_enrollment_detail(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<EnrollmentResponse>>, AppError> {
    let enrollment = s
        .svc
        .get_enrollment(id)
        .await
        .map_err(|e| AppError::NotFound(e.to_string()))?;
    Ok(Json(ApiResponse::ok(enrollment)))
}

pub async fn admin_enrollment_review(
    State(s): State<AppState>,
    Extension(claims): Extension<AuthClaims>,
    Path(id): Path<Uuid>,
    Json(body): Json<ReviewEnrollmentInput>,
) -> Result<Json<ApiResponse<EnrollmentResponse>>, AppError> {
    let result = s
        .svc
        .admin_review_enrollment(
            claims.user_id,
            id,
            body,
            s.notifier.as_deref(),
            s.auth_client.as_deref(),
        )
        .await
        .map_err(|e| AppError::Validation(e.to_string()))?;
    Ok(Json(ApiResponse::ok(result)))
}

pub async fn admin_enrollment_export_csv(
    State(s): State<AppState>,
    Query(q): Query<EnrollmentListQuery>,
) -> Result<Response, AppError> {
    let items = s
        .svc
        .admin_enrollment_export_csv(q)
        .await
        .map_err(AppError::Internal)?;
    let mut csv = String::from("ID,Pelatihan_ID,User_ID,Bukti_Transfer,Status,Reviewed_By,Review_Note,Created_At,Updated_At\n");
    for item in &items {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{}\n",
            item.id,
            item.pelatihan_id,
            item.user_id,
            item.bukti_transfer_object_key.as_deref().unwrap_or(""),
            item.status,
            item.reviewed_by.map(|v| v.to_string()).unwrap_or_default(),
            item.review_note
                .as_deref()
                .map(escape_csv)
                .unwrap_or_default(),
            item.created_at,
            item.updated_at
        ));
    }
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
        .header(
            header::CONTENT_DISPOSITION,
            "attachment; filename=enrollments.csv",
        )
        .body(axum::body::Body::from(csv))
        .unwrap())
}

// ═══════════════════════════════════════════════════════════════════════════
// Badge (user endpoints)
// ═══════════════════════════════════════════════════════════════════════════

/// User minta presigned URL sertifikat
pub async fn request_badge_evidence(
    State(s): State<AppState>,
    Extension(claims): Extension<AuthClaims>,
    ValidatedJson(body): ValidatedJson<BadgeEvidenceInput>,
) -> Result<Json<ApiResponse<storage_service_client::UploadPermission>>, AppError> {
    let storage = s
        .storage
        .as_ref()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("storage tidak tersedia")))?;
    let perm = s
        .svc
        .request_badge_evidence(storage.as_ref(), claims.user_id, body)
        .await
        .map_err(AppError::Internal)?;
    Ok(Json(ApiResponse::ok(perm)))
}

/// User mengajukan badge (POST /pelatihan/{id}/badge)
pub async fn create_badge(
    State(s): State<AppState>,
    Extension(claims): Extension<AuthClaims>,
    Path(pelatihan_id): Path<Uuid>,
) -> Result<(StatusCode, Json<ApiResponse<BadgeResponse>>), AppError> {
    let badge = s
        .svc
        .create_badge(pelatihan_id, claims.user_id)
        .await
        .map_err(|e| AppError::NotFound(e.to_string()))?;
    Ok((StatusCode::CREATED, Json(ApiResponse::ok(badge))))
}

/// User commit sertifikat (POST /pelatihan/badges/{id}/commit-sertifikat)
pub async fn commit_badge_sertifikat(
    State(s): State<AppState>,
    Extension(claims): Extension<AuthClaims>,
    Path(id): Path<Uuid>,
    Json(body): Json<CommitBadgeSertifikatInput>,
) -> Result<Json<ApiResponse<BadgeResponse>>, AppError> {
    let updated = s
        .svc
        .commit_badge_sertifikat(id, claims.user_id, body)
        .await
        .map_err(|e| AppError::Validation(e.to_string()))?;
    Ok(Json(ApiResponse::ok(updated)))
}

// ═══════════════════════════════════════════════════════════════════════════
// Admin: badge management
// ═══════════════════════════════════════════════════════════════════════════

pub async fn admin_badge_list(
    State(s): State<AppState>,
    Query(q): Query<BadgeListQuery>,
) -> Result<Json<ApiResponse<Vec<BadgeResponse>>>, AppError> {
    let (items, total) = s
        .svc
        .admin_badge_list(q)
        .await
        .map_err(AppError::Internal)?;
    let meta = common_errors::PaginatedMeta::new(1, 20, total);
    Ok(Json(ApiResponse::with_meta(items, meta)))
}

pub async fn admin_badge_detail(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<BadgeResponse>>, AppError> {
    let badge = s
        .svc
        .get_badge(id)
        .await
        .map_err(|e| AppError::NotFound(e.to_string()))?;
    Ok(Json(ApiResponse::ok(badge)))
}

pub async fn admin_badge_review(
    State(s): State<AppState>,
    Extension(claims): Extension<AuthClaims>,
    Path(id): Path<Uuid>,
    Json(body): Json<ReviewBadgeInput>,
) -> Result<Json<ApiResponse<BadgeResponse>>, AppError> {
    let result = s
        .svc
        .admin_review_badge(
            claims.user_id,
            id,
            body,
            s.notifier.as_deref(),
            s.auth_client.as_deref(),
        )
        .await
        .map_err(|e| AppError::Validation(e.to_string()))?;
    Ok(Json(ApiResponse::ok(result)))
}

pub async fn admin_badge_export_csv(
    State(s): State<AppState>,
    Query(q): Query<BadgeListQuery>,
) -> Result<Response, AppError> {
    let items = s
        .svc
        .admin_badge_export_csv(q)
        .await
        .map_err(AppError::Internal)?;
    let mut csv = String::from("ID,Pelatihan_ID,User_ID,Sertifikat,Approved_At,Status,Reviewed_By,Review_Note,Created_At,Updated_At\n");
    for item in &items {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{}\n",
            item.id,
            item.pelatihan_id,
            item.user_id,
            item.sertifikat_object_key.as_deref().unwrap_or(""),
            item.approved_at.map(|v| v.to_string()).unwrap_or_default(),
            item.status,
            item.reviewed_by.map(|v| v.to_string()).unwrap_or_default(),
            item.review_note
                .as_deref()
                .map(escape_csv)
                .unwrap_or_default(),
            item.created_at,
            item.updated_at
        ));
    }
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
        .header(
            header::CONTENT_DISPOSITION,
            "attachment; filename=badges.csv",
        )
        .body(axum::body::Body::from(csv))
        .unwrap())
}

// ── CSV helper ───────────────────────────────────────────────────────────
// Formula injection prevention: if the cell starts with =, +, -, or @,
// prepend a tab character to neuter spreadsheet formula interpretation.
// Ref: OWASP CSV Injection (CWE-1236)

fn escape_csv(s: &str) -> String {
    let needs_quote = s.contains(',') || s.contains('"') || s.contains('\n');
    let escaped = if s.contains('"') {
        s.replace('"', "\"\"")
    } else {
        s.to_string()
    };
    // Prevent CSV formula injection
    let safe = if let Some(first) = s.chars().next() {
        if first == '=' || first == '+' || first == '-' || first == '@' {
            format!("\t{}", escaped)
        } else {
            escaped
        }
    } else {
        escaped
    };
    if needs_quote {
        format!("\"{}\"", safe)
    } else {
        safe
    }
}
