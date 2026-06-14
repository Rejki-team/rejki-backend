use super::AppState;
use crate::application::dto::{
    CreateReportInput, ReportDetailResponse, ReportListQuery, ReportResponse, ReviewReportInput,
};
use crate::application::service::to_report_resp;
use auth_service_client::AuthClaims;
use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::Response,
    Extension, Json,
};
use common_errors::{created_response, ApiResponse, AppError, PaginatedMeta, ValidatedJson};
use notification_service_client::{EmailMessage, NotificationPayload};
use report_service_client::ReportTargetType;
use std::sync::Arc;
use uuid::Uuid;

// ═══════════════════════════════════════════════════════════════════════════
// User: create report (mobile)
// ═══════════════════════════════════════════════════════════════════════════

pub async fn create_report(
    State(s): State<AppState>,
    Extension(claims): Extension<AuthClaims>,
    ValidatedJson(body): ValidatedJson<CreateReportInput>,
) -> Result<Response, AppError> {
    let target_type = ReportTargetType::parse(&body.target_type)
        .ok_or_else(|| AppError::Validation("target_type harus 'iklan' atau 'user'".into()))?;

    // Upload evidence jika disediakan
    let evidence_key = match (body.mime, body.size_bytes) {
        (Some(mime), Some(size_bytes)) => {
            let storage = s
                .storage
                .as_ref()
                .ok_or_else(|| AppError::Internal(anyhow::anyhow!("storage tidak tersedia")))?;

            let permission = storage
                .request_upload(
                    crate::application::service::storage_category::REPORT_EVIDENCE,
                    claims.user_id,
                    storage_service_client::FileInfo { mime, size_bytes },
                )
                .await
                .map_err(|e| AppError::Validation(format!("bukti tidak valid: {e}")))?;

            Some(permission.object_key)
        }
        _ => None,
    };

    let report = s
        .svc
        .create_report(
            claims.user_id,
            target_type,
            body.target_id,
            body.keterangan,
            evidence_key,
        )
        .await
        .map_err(AppError::Internal)?;

    let id = report.id;
    Ok(created_response(
        ApiResponse::ok(to_report_resp(report)),
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

    let resp = to_report_resp(report.clone());

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

// ═══════════════════════════════════════════════════════════════════════════
// Admin: export CSV
// ═══════════════════════════════════════════════════════════════════════════

pub async fn admin_export_csv(
    State(s): State<AppState>,
    Query(q): Query<ReportListQuery>,
) -> Result<(StatusCode, [(header::HeaderName, &'static str); 2], String), AppError> {
    let items = s.svc.admin_list_all(q).await.map_err(AppError::Internal)?;

    let mut csv = String::from("ID,ReporterID,TargetType,TargetID,Keterangan,Status,ActionNote,ReviewedBy,CreatedAt,UpdatedAt\n");
    for r in &items {
        csv.push_str(&format!(
            "{},{},{},{},\"{}\",{},{},{},{},{}\n",
            r.id,
            r.reporter_id,
            r.target_type.as_str(),
            r.target_id,
            r.keterangan.replace('"', "\"\""),
            r.status.as_str(),
            r.action_note.as_deref().unwrap_or(""),
            r.reviewed_by.map(|u| u.to_string()).unwrap_or_default(),
            r.created_at,
            r.updated_at,
        ));
    }

    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"reports.csv\"",
            ),
        ],
        csv,
    ))
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
