use super::AppState;
use crate::application::dto::{
    AdminListQuery, CreateIklanPekerjaInput, IklanPekerjaResponse, ListQuery, SuspendEvidenceInput,
    SuspendInput, SuspendResponse, UpdatePekerjaInput,
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

pub async fn list(
    State(s): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<ApiResponse<Vec<IklanPekerjaResponse>>>, AppError> {
    Ok(Json(ApiResponse::ok(
        s.svc.list(q).await.map_err(AppError::Internal)?,
    )))
}

pub async fn get_by_id(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<IklanPekerjaResponse>>, AppError> {
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
    ValidatedJson(body): ValidatedJson<CreateIklanPekerjaInput>,
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
        &format!("/api/v1/pekerja/{id}"),
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

pub async fn update(
    State(s): State<AppState>,
    Extension(claims): Extension<AuthClaims>,
    Path(id): Path<Uuid>,
    ValidatedJson(body): ValidatedJson<UpdatePekerjaInput>,
) -> Result<Json<ApiResponse<IklanPekerjaResponse>>, AppError> {
    let item = s.svc.update(claims.user_id, id, body).await.map_err(|e| {
        let msg = e.to_string();
        if msg.contains("terlalu banyak permintaan") {
            AppError::RateLimited(msg)
        } else if msg == "tidak ditemukan" {
            AppError::NotFound(msg)
        } else if msg == "iklan tidak dapat diubah dalam status moderasi saat ini" {
            AppError::Conflict(msg)
        } else if msg.contains("region_id tidak ditemukan") {
            AppError::Validation(msg)
        } else {
            AppError::Internal(e)
        }
    })?;
    Ok(Json(ApiResponse::ok(item)))
}

pub async fn health() -> (StatusCode, Json<serde_json::Value>) {
    (StatusCode::OK, Json(serde_json::json!({ "status": "ok" })))
}

pub async fn admin_list(
    State(s): State<AppState>,
    Query(q): Query<AdminListQuery>,
) -> Result<Json<ApiResponse<Vec<crate::application::dto::AdminIklanPekerjaResponse>>>, AppError> {
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
    let mut csv = String::from("ID,Nama,Keahlian,Deskripsi,Lokasi,Tarif_Min,Tarif_Max,Status_Moderasi,Pembuat,Created_At\n");
    for item in &items {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{}\n",
            item.id,
            escape_csv(&item.nama),
            escape_csv(&item.keahlian.join("; ")),
            escape_csv(&item.deskripsi),
            item.lokasi.as_deref().unwrap_or(""),
            item.tarif_min.map(|v| v.to_string()).unwrap_or_default(),
            item.tarif_max.map(|v| v.to_string()).unwrap_or_default(),
            item.moderation_status,
            item.poster_id,
            item.created_at
        ));
    }
    Ok(csv_response(csv, "iklan_pekerja.csv"))
}

/// GET /admin/{id} — detail iklan untuk pop-up admin, termasuk indikator dokumen
/// sensitif poster (F-27b).
pub async fn admin_get_detail(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<crate::application::dto::AdminIklanPekerjaDetailResponse>>, AppError> {
    let detail = s
        .svc
        .admin_get_detail(id)
        .await
        .map_err(AppError::Internal)?
        .ok_or_else(|| AppError::NotFound("iklan tidak ditemukan".into()))?;
    Ok(Json(ApiResponse::ok(detail)))
}

/// GET /admin/{id}/sensitive/{kind} — proxy reveal data sensitif poster (nik|ktp|selfie)
/// ke user-service via UserClient. Audit tercatat tunggal di user-service (F-27b).
pub async fn admin_reveal_sensitive(
    State(s): State<AppState>,
    Extension(claims): Extension<AuthClaims>,
    Path((id, kind)): Path<(Uuid, String)>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    if !["nik", "ktp", "selfie"].contains(&kind.as_str()) {
        return Err(AppError::Validation(
            "jenis data harus 'nik', 'ktp', atau 'selfie'".into(),
        ));
    }

    let value = s
        .svc
        .admin_reveal_sensitive(id, &kind, claims.user_id)
        .await
        .map_err(AppError::Internal)?;

    match value {
        Some(v) => {
            let key = if kind == "nik" { "nik" } else { "url" };
            Ok(Json(ApiResponse::ok(serde_json::json!({ key: v }))))
        }
        None => Err(AppError::NotFound("data tidak tersedia".into())),
    }
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

fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}
