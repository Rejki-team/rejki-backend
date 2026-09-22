use super::AppState;
use crate::application::dto::{
    ApproveAndSuspendInput, CreateLaporkanIklanInput, CreatePelaporanMasalahInput,
    CreatePelaporanMasalahResponse, ReportDetailResponse, ReportListQuery, ReportResponse,
    ReviewReportInput,
};
use crate::application::service::to_report_resp;
use auth_service_client::AuthClaims;
use axum::{
    extract::{Path, Query, State},
    response::Response,
    Extension, Json,
};
use common_errors::{
    created_response, csv_response, ApiResponse, AppError, PaginatedMeta, ValidatedJson,
};
use notification_service_client::{EmailMessage, NotificationPayload};
use report_service_client::{ReportAdType, ReportTargetType};
use std::sync::Arc;
use uuid::Uuid;

fn map_create_error(e: anyhow::Error) -> AppError {
    let msg = e.to_string();
    if msg.contains("terlalu banyak permintaan") {
        AppError::RateLimited(msg)
    } else {
        AppError::Internal(e)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// User: create report — jalur 1 "Laporkan Iklan" (mobile, P1.4)
// ═══════════════════════════════════════════════════════════════════════════

pub async fn create_laporkan_iklan(
    State(s): State<AppState>,
    Extension(claims): Extension<AuthClaims>,
    ValidatedJson(body): ValidatedJson<CreateLaporkanIklanInput>,
) -> Result<Response, AppError> {
    let target_type = ReportTargetType::parse(&body.target_type)
        .ok_or_else(|| AppError::Validation("target_type harus 'iklan' atau 'user'".into()))?;
    let target_ad_type = body
        .target_ad_type
        .as_deref()
        .map(|s| {
            ReportAdType::parse(s).ok_or_else(|| {
                AppError::Validation(
                    "target_ad_type harus 'pekerjaan', 'pekerja', atau 'barang_bekas'".into(),
                )
            })
        })
        .transpose()?;

    let report = s
        .svc
        .create_laporkan_iklan(
            claims.user_id,
            target_type,
            target_ad_type,
            body.target_id,
            body.keterangan,
        )
        .await
        .map_err(map_create_error)?;

    let id = report.id;
    Ok(created_response(
        ApiResponse::ok(to_report_resp(report, None)),
        &format!("/api/v1/reports/{}", id),
    ))
}

// ═══════════════════════════════════════════════════════════════════════════
// User: create report — jalur 2 "Pelaporan Masalah" (mobile, P1.4)
// ═══════════════════════════════════════════════════════════════════════════

pub async fn create_pelaporan_masalah(
    State(s): State<AppState>,
    Extension(claims): Extension<AuthClaims>,
    ValidatedJson(body): ValidatedJson<CreatePelaporanMasalahInput>,
) -> Result<Response, AppError> {
    // Bukti gambar WAJIB (PRD §6.10 tabel perbedaan kelengkapan data) — divalidasi
    // di DTO (min=1 mime, max 300KB) DAN di sini via request_upload storage.
    let storage = s
        .storage
        .as_ref()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("storage tidak tersedia")))?;
    let permission = storage
        .request_upload(
            crate::application::service::storage_category::REPORT_EVIDENCE,
            claims.user_id,
            storage_service_client::FileInfo {
                mime: body.mime,
                size_bytes: body.size_bytes,
            },
        )
        .await
        .map_err(|e| AppError::Validation(format!("bukti tidak valid: {e}")))?;

    let report = s
        .svc
        .create_pelaporan_masalah(
            claims.user_id,
            body.target_id,
            body.keterangan,
            permission.object_key,
        )
        .await
        .map_err(map_create_error)?;

    let id = report.id;
    Ok(created_response(
        ApiResponse::ok(CreatePelaporanMasalahResponse {
            report: to_report_resp(report, None),
            presigned_url: permission.presigned_url,
        }),
        &format!("/api/v1/reports/{}", id),
    ))
}

// ═══════════════════════════════════════════════════════════════════════════
// Admin: list reports
// ═══════════════════════════════════════════════════════════════════════════

pub async fn admin_list(
    State(s): State<AppState>,
    Query(q): Query<ReportListQuery>,
) -> Result<Json<ApiResponse<Vec<ReportResponse>>>, AppError> {
    let limit = q.limit.unwrap_or(crate::domain::repository::DEFAULT_LIMIT);
    let offset = q.offset.unwrap_or(0);
    let (items, total) = s.svc.admin_list(q).await.map_err(AppError::Internal)?;
    let page = if limit > 0 {
        (offset / limit) as u32 + 1
    } else {
        1u32
    };
    let per_page = limit.max(1) as u32;
    Ok(Json(ApiResponse::with_meta(
        items,
        PaginatedMeta::new(page, per_page, total),
    )))
}

// ═══════════════════════════════════════════════════════════════════════════
// Admin: get detail
// ═══════════════════════════════════════════════════════════════════════════

pub async fn admin_get(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<ReportDetailResponse>>, AppError> {
    let mut detail = s
        .svc
        .admin_get_detail(id)
        .await
        .map_err(|e| AppError::NotFound(e.to_string()))?;

    // Generate presigned read URL untuk bukti (bila ada)
    if let Some(ref object_key) = detail.evidence_object_key {
        if let Some(ref storage) = s.storage {
            match storage.request_download(object_key).await {
                Ok(url) => detail.evidence_read_url = Some(url),
                Err(e) => {
                    tracing::warn!(object_key = %object_key, error = ?e, "gagal generate presigned read URL")
                }
            }
        }
    }

    Ok(Json(ApiResponse::ok(detail)))
}

// ═══════════════════════════════════════════════════════════════════════════
// Admin: review (approve/reject)
// ═══════════════════════════════════════════════════════════════════════════

pub async fn admin_review(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
    Extension(claims): Extension<AuthClaims>,
    ValidatedJson(body): ValidatedJson<ReviewReportInput>,
) -> Result<Json<ApiResponse<ReportResponse>>, AppError> {
    let report = s
        .svc
        .admin_review(id, body.approved, body.action_note, claims.user_id)
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("sudah ditindaklanjuti") {
                AppError::Conflict(msg)
            } else {
                AppError::NotFound(msg)
            }
        })?;

    let resp = to_report_resp(report.clone(), None);

    // Fire-and-forget notifikasi ke pelapor
    if let Some(ref notifier) = s.notifier {
        let notifier = notifier.clone();
        let auth_client = s.auth_client.clone();
        let reporter_id = report.reporter_id;
        let status = report.status.as_str().to_string();
        let action_note = report.action_note.clone().unwrap_or_default();
        tokio::spawn(async move {
            notify_reporter(auth_client, notifier, reporter_id, id, status, action_note).await;
        });
    }

    Ok(Json(ApiResponse::ok(resp)))
}

/// Endpoint terintegrasi "Terima & Suspend" (P9.1, Kelompok 6 Q9) — approve
/// aduan sekaligus suspend target dalam 1 aksi. Endpoint suspend mandiri
/// existing di masing-masing halaman admin (Iklan/Pengguna) TETAP ada tanpa
/// perubahan (P9.2) — ini murni opsi tambahan.
pub async fn admin_approve_and_suspend(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
    Extension(claims): Extension<AuthClaims>,
    ValidatedJson(body): ValidatedJson<ApproveAndSuspendInput>,
) -> Result<Json<ApiResponse<ReportResponse>>, AppError> {
    let report = s
        .svc
        .admin_approve_and_suspend(
            id,
            body,
            claims.user_id,
            s.auth_client.as_ref(),
            s.iklan_pekerjaan_client.as_deref(),
            s.iklan_pekerja_client.as_deref(),
            s.iklan_barang_bekas_client.as_deref(),
        )
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("sudah ditindaklanjuti") {
                AppError::Conflict(msg)
            } else if msg.contains("tidak tersedia") || msg.contains("tidak diketahui") {
                AppError::Internal(anyhow::anyhow!(msg))
            } else {
                AppError::NotFound(msg)
            }
        })?;

    let resp = to_report_resp(report.clone(), None);

    // Fire-and-forget notifikasi ke pelapor (pola sama admin_review).
    if let Some(ref notifier) = s.notifier {
        let notifier = notifier.clone();
        let auth_client = s.auth_client.clone();
        let reporter_id = report.reporter_id;
        let status = report.status.as_str().to_string();
        let action_note = report.action_note.clone().unwrap_or_default();
        tokio::spawn(async move {
            notify_reporter(auth_client, notifier, reporter_id, id, status, action_note).await;
        });
    }

    Ok(Json(ApiResponse::ok(resp)))
}

// ═══════════════════════════════════════════════════════════════════════════
// Admin: export CSV
// ═══════════════════════════════════════════════════════════════════════════

pub async fn admin_export_csv(
    State(s): State<AppState>,
    Query(q): Query<ReportListQuery>,
) -> Result<Response, AppError> {
    let items = s.svc.admin_list_all(q).await.map_err(AppError::Internal)?;

    let mut csv = String::from("ID,ReporterID,ReportType,TargetType,TargetID,Keterangan,Status,DueDate,IsOverdue,ActionNote,ReviewedBy,CreatedAt,UpdatedAt\n");
    for r in &items {
        csv.push_str(&format!(
            "{},{},{},{},{},\"{}\",{},{},{},{},{},{},{}\n",
            r.id,
            r.reporter_id,
            r.report_type.as_str(),
            r.target_type.map(|t| t.as_str()).unwrap_or(""),
            r.target_id.map(|t| t.to_string()).unwrap_or_default(),
            r.keterangan.replace('"', "\"\""),
            r.status.as_str(),
            r.due_date,
            r.is_overdue,
            r.action_note.as_deref().unwrap_or(""),
            r.reviewed_by.map(|u| u.to_string()).unwrap_or_default(),
            r.created_at,
            r.updated_at,
        ));
    }

    Ok(csv_response(csv, "reports.csv"))
}

// ═══════════════════════════════════════════════════════════════════════════
// Notification helper — fire-and-forget, error di-log
// ═══════════════════════════════════════════════════════════════════════════

async fn notify_reporter(
    auth_client: Arc<dyn auth_service_client::AuthClient>,
    notifier: Arc<dyn notification_service_client::NotificationClient>,
    reporter_id: Uuid,
    report_id: Uuid,
    status: String,
    action_note: String,
) {
    let status_label = match status.as_str() {
        "resolved" => "disetujui",
        "rejected" => "ditolak",
        _ => "ditinjau",
    };

    // In-app notification
    let payload = NotificationPayload {
        title: format!("Aduan {}", status_label),
        body: format!(
            "Aduan #{} telah {}. Tindakan: {}",
            report_id, status_label, action_note
        ),
        data: Some(serde_json::json!({
            "type": "report_reviewed",
            "report_id": report_id.to_string(),
            "status": status,
        })),
    };

    if let Err(e) = notifier.send(reporter_id, payload).await {
        tracing::error!(error = ?e, report_id = %report_id, "gagal kirim notifikasi in-app hasil aduan");
    }

    // Email notification
    if let Ok(email) = auth_client.get_account_email(reporter_id).await {
        let email_msg = EmailMessage {
            to: email,
            subject: format!("Aduan #{} {}", report_id, status_label),
            body: format!(
                "Aduan Anda telah {}. Tindakan yang diambil: {}",
                status_label, action_note
            ),
        };
        if let Err(e) = notifier.send_email(email_msg).await {
            tracing::error!(error = ?e, report_id = %report_id, "gagal kirim email hasil aduan");
        }
    }
}
