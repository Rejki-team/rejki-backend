use super::AppState;
use crate::application::dto::{
    AdminArticleResponse, ArticleListQuery, ArticlePhotoInput, CreateArticleInput,
    UpdateArticleInput,
};
use crate::application::service::to_admin_resp;
use auth_service_client::{AuthClaims, AuthClient};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Response,
    Extension, Json,
};
use common_errors::{created_response, ApiResponse, AppError, PaginatedMeta, ValidatedJson};
use notification_service_client::{NotificationClient, NotificationPayload};
use std::sync::Arc;
use uuid::Uuid;

// ═══════════════════════════════════════════════════════════════════════════
// Admin listing
// ═══════════════════════════════════════════════════════════════════════════

pub async fn admin_list(
    State(s): State<AppState>,
    Query(q): Query<ArticleListQuery>,
) -> Result<Json<ApiResponse<Vec<AdminArticleResponse>>>, AppError> {
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
// Admin get by id
// ═══════════════════════════════════════════════════════════════════════════

pub async fn admin_get(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<AdminArticleResponse>>, AppError> {
    Ok(Json(ApiResponse::ok(
        s.svc
            .admin_get(id)
            .await
            .map_err(|e| AppError::NotFound(e.to_string()))?,
    )))
}

// ═══════════════════════════════════════════════════════════════════════════
// Admin create article
// ═══════════════════════════════════════════════════════════════════════════

pub async fn admin_create(
    State(s): State<AppState>,
    Extension(claims): Extension<AuthClaims>,
    ValidatedJson(body): ValidatedJson<CreateArticleInput>,
) -> Result<Response, AppError> {
    let article = s
        .svc
        .admin_create(claims.user_id, body)
        .await
        .map_err(AppError::Internal)?;

    let id = article.id;
    let resp = to_admin_resp(article.clone());

    // Fire-and-forget broadcast ke seluruh pengguna aktif via AuthClient
    if let Some(ref notifier) = s.notifier {
        let auth_client = s.auth_client.clone();
        let notifier = notifier.clone();
        let title = article.title.clone();
        let article_id = article.id;
        tokio::spawn(async move {
            broadcast_to_all(auth_client, notifier, article_id, title, "baru").await;
        });
    }

    Ok(created_response(
        ApiResponse::ok(resp),
        &format!("/api/v1/admin/articles/{id}"),
    ))
}

// ═══════════════════════════════════════════════════════════════════════════
// Admin update article
// ═══════════════════════════════════════════════════════════════════════════

pub async fn admin_update(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
    ValidatedJson(body): ValidatedJson<UpdateArticleInput>,
) -> Result<Json<ApiResponse<AdminArticleResponse>>, AppError> {
    let article = s
        .svc
        .admin_update(id, body)
        .await
        .map_err(AppError::Internal)?;

    let resp = to_admin_resp(article.clone());

    // Fire-and-forget broadcast ke seluruh pengguna aktif via AuthClient
    if let Some(ref notifier) = s.notifier {
        let auth_client = s.auth_client.clone();
        let notifier = notifier.clone();
        let title = article.title.clone();
        let article_id = article.id;
        tokio::spawn(async move {
            broadcast_to_all(auth_client, notifier, article_id, title, "diperbarui").await;
        });
    }

    Ok(Json(ApiResponse::ok(resp)))
}

// ═══════════════════════════════════════════════════════════════════════════
// Admin delete (soft)
// ═══════════════════════════════════════════════════════════════════════════

pub async fn admin_delete(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let deleted = s.svc.admin_delete(id).await.map_err(AppError::Internal)?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound("artikel tidak ditemukan".into()))
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Photo upload
// ═══════════════════════════════════════════════════════════════════════════

pub async fn request_photo_upload(
    State(s): State<AppState>,
    Extension(claims): Extension<AuthClaims>,
    ValidatedJson(body): ValidatedJson<ArticlePhotoInput>,
) -> Result<Json<ApiResponse<storage_service_client::UploadPermission>>, AppError> {
    let storage = s
        .storage
        .as_ref()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("storage tidak tersedia")))?;
    let permission = s
        .svc
        .request_article_photo_upload(storage.as_ref(), claims.user_id, body)
        .await
        .map_err(AppError::Internal)?;
    Ok(Json(ApiResponse::ok(permission)))
}

// ═══════════════════════════════════════════════════════════════════════════
// Broadcast helper — fire-and-forget, error di-log
// Menggunakan AuthClient::list_active_user_ids() agar Zero Cross-Schema Query.
// ═══════════════════════════════════════════════════════════════════════════

async fn broadcast_to_all(
    auth_client: Arc<dyn AuthClient>,
    notifier: Arc<dyn NotificationClient>,
    article_id: Uuid,
    title: String,
    action: &str,
) {
    let user_ids = match auth_client.list_active_user_ids().await {
        Ok(ids) => ids,
        Err(e) => {
            tracing::error!(error = ?e, "gagal mengambil daftar user_id untuk broadcast artikel");
            return;
        }
    };

    if user_ids.is_empty() {
        tracing::info!("tidak ada pengguna aktif untuk broadcast artikel");
        return;
    }

    let payload = NotificationPayload {
        title: title.clone(),
        body: format!("Artikel {}: {}", action, title),
        data: Some(serde_json::json!({
            "type": "corporate_article",
            "article_id": article_id.to_string(),
            "action": action,
        })),
    };

    if let Err(e) = notifier.send_bulk(user_ids, payload).await {
        tracing::error!(error = ?e, article_id = %article_id, "gagal broadcast notifikasi artikel");
    } else {
        tracing::info!(article_id = %article_id, action = action, "broadcast notifikasi artikel berhasil");
    }
}
