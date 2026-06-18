use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use auth_service_client::AuthClaims;
use common_errors::{ApiResponse, AppError, ValidatedJson};

use super::AppState;
use crate::application::dto::{
    ConversationResponse, ListMessagesQuery, MessageResponse, SendMessageInput,
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

    let conv = s
        .chat_svc
        .get_or_create_conversation(claims.user_id, other_id)
        .await
        .map_err(AppError::Internal)?;

    Ok(Json(ApiResponse::ok(conv)))
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
    ValidatedJson(body): ValidatedJson<SendMessageInput>,
) -> Result<(StatusCode, Json<ApiResponse<MessageResponse>>), AppError> {
    let msg = s
        .chat_svc
        .send_message(conv_id, claims.user_id, body)
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("terlalu banyak permintaan") {
                AppError::RateLimited(msg)
            } else {
                AppError::Validation(msg)
            }
        })?;

    Ok((StatusCode::CREATED, Json(ApiResponse::ok(msg))))
}
