use super::AppState;
use crate::application::dto::{
    AdminListQuery, BatalkanLamaranInput, CreateIklanPekerjaanInput, IklanPekerjaanResponse,
    LamarInput, LamaranResponse, LamaranWithIklanResponse, LamaranWithPelamarResponse, ListQuery,
    MulaiBekerjaInput, ReviewLamaranInput, SuspendEvidenceInput, SuspendInput, SuspendResponse,
    UpdatePekerjaanInput,
};
use auth_service_client::AuthClaims;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Response,
    Extension, Json,
};
use common_errors::{created_response, csv_response, ApiResponse, AppError, ValidatedJson};
use uuid::Uuid;

// ── Public handlers ──────────────────────────────────────────────────────────

pub async fn list(
    State(s): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<ApiResponse<Vec<IklanPekerjaanResponse>>>, AppError> {
    Ok(Json(ApiResponse::ok(
        s.svc.list(q).await.map_err(AppError::Internal)?,
    )))
}

/// "Iklan Saya" (Kelompok 3 Phase 2) — entry point ke "Kelola Pelamar".
pub async fn list_my_jobs(
    State(s): State<AppState>,
    Extension(claims): Extension<AuthClaims>,
    Query(q): Query<ListQuery>,
) -> Result<Json<ApiResponse<Vec<IklanPekerjaanResponse>>>, AppError> {
    Ok(Json(ApiResponse::ok(
        s.svc
            .list_my_jobs(claims.user_id, q)
            .await
            .map_err(AppError::Internal)?,
    )))
}

pub async fn get_by_id(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<IklanPekerjaanResponse>>, AppError> {
    Ok(Json(ApiResponse::ok(
        s.svc
            .get(id)
            .await
            .map_err(|e| AppError::NotFound(e.to_string()))?,
    )))
}

pub async fn create(
    State(s): State<AppState>,
    Extension(claims): Extension<AuthClaims>,
    ValidatedJson(body): ValidatedJson<CreateIklanPekerjaanInput>,
) -> Result<Response, AppError> {
    let item = s.svc.create(claims.user_id, body).await.map_err(|e| {
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
        &format!("/api/v1/pekerjaan/{id}"),
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
        Err(AppError::NotFound("iklan tidak ditemukan".into()))
    }
}

pub async fn update_iklan(
    State(s): State<AppState>,
    Extension(claims): Extension<AuthClaims>,
    Path(id): Path<Uuid>,
    ValidatedJson(body): ValidatedJson<UpdatePekerjaanInput>,
) -> Result<Json<ApiResponse<IklanPekerjaanResponse>>, AppError> {
    let item = s.svc.update(claims.user_id, id, body).await.map_err(|e| {
        let msg = e.to_string();
        if msg.contains("terlalu banyak permintaan") {
            AppError::RateLimited(msg)
        } else if msg.contains("tidak ditemukan") {
            AppError::NotFound(msg)
        } else if msg.contains("status moderasi") {
            AppError::Conflict(msg)
        } else {
            AppError::Internal(e)
        }
    })?;
    Ok(Json(ApiResponse::ok(item)))
}

/// Health check service (tanpa auth).
pub async fn health() -> (StatusCode, Json<serde_json::Value>) {
    (StatusCode::OK, Json(serde_json::json!({ "status": "ok" })))
}

// ── Lamaran (F-3, Kelompok 3 Phase 1) ────────────────────────────────────────

/// Pemetaan pesan error `anyhow` dari `IklanPekerjaanService` alur Lamaran ke `AppError`
/// HTTP yang tepat — pola sama dengan `update_iklan` (match substring, bukan error enum
/// terpisah, karena error berasal dari lintas layer domain/infrastructure).
fn map_lamaran_error(e: anyhow::Error) -> AppError {
    let msg = e.to_string();
    if msg.contains("tidak ditemukan") {
        AppError::NotFound(msg)
    } else if msg.contains("milik sendiri")
        || msg.contains("Iklan Pekerja aktif")
        || msg.contains("luar radius 50m")
    {
        AppError::Validation(msg)
    } else if msg.contains("rentang waktu yang sama")
        || msg.contains("jam sebelum pekerjaan dimulai")
        || msg.contains("tidak dapat dimulai")
        || msg.contains("tidak dapat ditandai selesai")
    {
        AppError::Conflict(msg)
    } else {
        AppError::Internal(e)
    }
}

pub async fn lamar(
    State(s): State<AppState>,
    Extension(claims): Extension<AuthClaims>,
    Path(iklan_id): Path<Uuid>,
    ValidatedJson(body): ValidatedJson<LamarInput>,
) -> Result<Response, AppError> {
    let item = s
        .svc
        .lamar(claims.user_id, iklan_id, body)
        .await
        .map_err(map_lamaran_error)?;
    let id = item.id;
    Ok(created_response(
        ApiResponse::ok(item),
        &format!("/api/v1/pekerjaan/{iklan_id}/lamaran/{id}"),
    ))
}

pub async fn review_lamaran(
    State(s): State<AppState>,
    Extension(claims): Extension<AuthClaims>,
    Path((_iklan_id, lamaran_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<ReviewLamaranInput>,
) -> Result<Json<ApiResponse<LamaranResponse>>, AppError> {
    let item = s
        .svc
        .review_lamaran(claims.user_id, lamaran_id, body)
        .await
        .map_err(map_lamaran_error)?;
    Ok(Json(ApiResponse::ok(item)))
}

pub async fn mulai_bekerja(
    State(s): State<AppState>,
    Extension(claims): Extension<AuthClaims>,
    Path((_iklan_id, lamaran_id)): Path<(Uuid, Uuid)>,
    ValidatedJson(body): ValidatedJson<MulaiBekerjaInput>,
) -> Result<Json<ApiResponse<LamaranResponse>>, AppError> {
    let item = s
        .svc
        .mulai_bekerja(claims.user_id, lamaran_id, body)
        .await
        .map_err(map_lamaran_error)?;
    Ok(Json(ApiResponse::ok(item)))
}

pub async fn tandai_selesai(
    State(s): State<AppState>,
    Extension(claims): Extension<AuthClaims>,
    Path((_iklan_id, lamaran_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ApiResponse<LamaranResponse>>, AppError> {
    let item = s
        .svc
        .tandai_selesai(claims.user_id, lamaran_id)
        .await
        .map_err(map_lamaran_error)?;
    Ok(Json(ApiResponse::ok(item)))
}

pub async fn batalkan_lamaran(
    State(s): State<AppState>,
    Extension(claims): Extension<AuthClaims>,
    Path((_iklan_id, lamaran_id)): Path<(Uuid, Uuid)>,
    ValidatedJson(body): ValidatedJson<BatalkanLamaranInput>,
) -> Result<Json<ApiResponse<LamaranResponse>>, AppError> {
    let item = s
        .svc
        .batalkan_lamaran(claims.user_id, lamaran_id, body)
        .await
        .map_err(map_lamaran_error)?;
    Ok(Json(ApiResponse::ok(item)))
}

pub async fn list_lamaran_for_iklan(
    State(s): State<AppState>,
    Extension(claims): Extension<AuthClaims>,
    Path(iklan_id): Path<Uuid>,
) -> Result<Json<ApiResponse<Vec<LamaranWithPelamarResponse>>>, AppError> {
    let items = s
        .svc
        .list_lamaran_for_iklan(claims.user_id, iklan_id)
        .await
        .map_err(map_lamaran_error)?;
    Ok(Json(ApiResponse::ok(items)))
}

pub async fn list_lamaran_saya(
    State(s): State<AppState>,
    Extension(claims): Extension<AuthClaims>,
) -> Result<Json<ApiResponse<Vec<LamaranWithIklanResponse>>>, AppError> {
    let items = s
        .svc
        .list_lamaran_for_pelamar(claims.user_id)
        .await
        .map_err(AppError::Internal)?;
    Ok(Json(ApiResponse::ok(items)))
}

// ── Admin: listing ───────────────────────────────────────────────────────────

pub async fn admin_list(
    State(s): State<AppState>,
    Query(q): Query<AdminListQuery>,
) -> Result<Json<ApiResponse<Vec<crate::application::dto::AdminIklanPekerjaanResponse>>>, AppError>
{
    let (items, total) = s.svc.admin_list(q).await.map_err(AppError::Internal)?;
    let meta = common_errors::PaginatedMeta::new(1, 20, total);
    Ok(Json(ApiResponse::with_meta(items, meta)))
}

// ── Admin: export CSV ────────────────────────────────────────────────────────

pub async fn admin_export_csv(
    State(s): State<AppState>,
    Query(q): Query<AdminListQuery>,
) -> Result<Response, AppError> {
    let items = s
        .svc
        .admin_export_csv(q)
        .await
        .map_err(AppError::Internal)?;

    // Generate CSV (kolom sesuai User Story)
    let mut csv = String::from("ID,Judul,Perusahaan,Deskripsi,Lokasi,Gaji_Min,Gaji_Max,Tipe,Status_Moderasi,Pembuat,Created_At\n");
    for item in &items {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{}\n",
            item.id,
            escape_csv(&item.judul),
            escape_csv(&item.perusahaan),
            escape_csv(&item.deskripsi),
            item.lokasi.as_deref().unwrap_or(""),
            item.gaji_min.map(|v| v.to_string()).unwrap_or_default(),
            item.gaji_max.map(|v| v.to_string()).unwrap_or_default(),
            item.tipe,
            item.moderation_status,
            item.poster_id,
            item.created_at
        ));
    }

    Ok(csv_response(csv, "iklan_pekerjaan.csv"))
}

// ── Admin: suspend evidence ──────────────────────────────────────────────────

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

// ── Admin: suspend ───────────────────────────────────────────────────────────

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

fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}
