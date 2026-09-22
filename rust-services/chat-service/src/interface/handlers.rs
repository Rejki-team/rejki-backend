use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use auth_service_client::AuthClaims;
use common_errors::{ApiResponse, AppError};

use super::AppState;
use crate::application::dto::{
    ConversationListItemResponse, ConversationResponse, ListMessagesQuery, MessageResponse,
    PhotoUploadPermissionResponse, SendMessageInput,
};

pub async fn get_or_create_conversation(
    State(s): State<AppState>,
    claims: axum::Extension<AuthClaims>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<ConversationResponse>>, AppError> {
    let other_id: Uuid = body["other_user_id"]
        .as_str()
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| AppError::Validation("other_user_id wajib diisi".into()))?;
    let related_ad_type = body["related_ad_type"].as_str();
    let related_ad_id = body["related_ad_id"]
        .as_str()
        .and_then(|s| Uuid::parse_str(s).ok());

    let conv = s
        .chat_svc
        .get_or_create_conversation(claims.user_id, other_id, related_ad_type, related_ad_id)
        .await
        .map_err(AppError::Internal)?;

    Ok(Json(ApiResponse::ok(conv)))
}

/// P4.10 — "Halaman daftar percakapan" (F-18, PRD §5.9).
pub async fn list_conversations(
    State(s): State<AppState>,
    claims: axum::Extension<AuthClaims>,
) -> Result<Json<ApiResponse<Vec<ConversationListItemResponse>>>, AppError> {
    let items = s
        .chat_svc
        .list_conversations(claims.user_id)
        .await
        .map_err(AppError::Internal)?;
    Ok(Json(ApiResponse::ok(items)))
}

/// P4.11 — tandai percakapan sudah dibaca (indikator belum dibaca, PRD §5.9).
pub async fn mark_read(
    State(s): State<AppState>,
    claims: axum::Extension<AuthClaims>,
    Path(conv_id): Path<Uuid>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    s.chat_svc
        .mark_read(conv_id, claims.user_id)
        .await
        .map_err(|e| AppError::NotFound(e.to_string()))?;
    Ok(Json(ApiResponse::ok(serde_json::json!({}))))
}

pub async fn list_messages(
    State(s): State<AppState>,
    Path(conv_id): Path<Uuid>,
    Query(query): Query<ListMessagesQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let (msgs, cursor) = s
        .chat_svc
        .list_messages(conv_id, query)
        .await
        .map_err(|e| AppError::NotFound(e.to_string()))?;

    let data = serde_json::json!({ "messages": msgs });
    Ok(if let Some(meta) = cursor {
        Json(ApiResponse::with_meta(data, meta))
    } else {
        Json(ApiResponse::ok(data))
    })
}

pub async fn send_message(
    State(s): State<AppState>,
    claims: axum::Extension<AuthClaims>,
    Path(conv_id): Path<Uuid>,
    Json(body): Json<SendMessageInput>,
) -> Result<(StatusCode, Json<ApiResponse<MessageResponse>>), AppError> {
    let msg = s
        .chat_svc
        .send_message(conv_id, claims.user_id, body)
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("terlalu banyak permintaan") {
                AppError::RateLimited(msg)
            } else if msg.contains("tidak ditemukan") {
                AppError::NotFound(msg)
            } else {
                AppError::Validation(msg)
            }
        })?;

    Ok((StatusCode::CREATED, Json(ApiResponse::ok(msg))))
}

/// P4.4 — "Tombol Akhiri Percakapan" manual (PRD §5.9).
pub async fn end_conversation(
    State(s): State<AppState>,
    claims: axum::Extension<AuthClaims>,
    Path(conv_id): Path<Uuid>,
) -> Result<Json<ApiResponse<ConversationResponse>>, AppError> {
    let conv = s
        .chat_svc
        .end_conversation(conv_id, claims.user_id)
        .await
        .map_err(|e| AppError::NotFound(e.to_string()))?;
    Ok(Json(ApiResponse::ok(conv)))
}

#[derive(Debug, Deserialize)]
pub struct PhotoUploadRequest {
    pub mime: String,
    pub size_bytes: u64,
}

/// P4.2 — minta presigned URL sebelum kirim pesan bertipe foto.
pub async fn request_photo_upload(
    State(s): State<AppState>,
    claims: axum::Extension<AuthClaims>,
    Json(body): Json<PhotoUploadRequest>,
) -> Result<Json<ApiResponse<PhotoUploadPermissionResponse>>, AppError> {
    let perm = s
        .chat_svc
        .request_photo_upload(claims.user_id, body.mime, body.size_bytes)
        .await
        .map_err(|e| AppError::Validation(e.to_string()))?;
    Ok(Json(ApiResponse::ok(perm)))
}
