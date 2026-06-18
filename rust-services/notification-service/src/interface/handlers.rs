use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use auth_service_client::AuthClaims;
use common_errors::{ApiResponse, AppError, ValidatedJson};

use super::AppState;
use crate::application::dto::{
    DeviceTokenResponse, NotificationResponse, RegisterDeviceTokenInput, SendNotificationInput,
};

pub async fn list_my_notifications(
    State(s): State<AppState>,
    claims: axum::Extension<AuthClaims>,
) -> Result<Json<ApiResponse<Vec<NotificationResponse>>>, AppError> {
    let notifs = s
        .notif_svc
        .list_for_user(claims.user_id, None)
        .await
        .map_err(AppError::Internal)?;

    Ok(Json(ApiResponse::ok(notifs)))
}

pub async fn send(
    State(s): State<AppState>,
    ValidatedJson(body): ValidatedJson<SendNotificationInput>,
) -> Result<StatusCode, AppError> {
    s.notif_svc.send(body).await.map_err(|e| {
        let msg = e.to_string();
        if msg.contains("terlalu banyak permintaan") {
            AppError::RateLimited(msg)
        } else {
            AppError::Internal(e)
        }
    })?;

    Ok(StatusCode::ACCEPTED)
}

pub async fn mark_read(
    State(s): State<AppState>,
    claims: axum::Extension<AuthClaims>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    s.notif_svc
        .mark_read(id, claims.user_id)
        .await
        .map_err(AppError::Internal)?;

    Ok(StatusCode::OK)
}

// ── Device token ──────────────────────────────────────────────────────────

pub async fn register_device_token(
    State(s): State<AppState>,
    claims: axum::Extension<AuthClaims>,
    ValidatedJson(body): ValidatedJson<RegisterDeviceTokenInput>,
) -> Result<(StatusCode, Json<ApiResponse<DeviceTokenResponse>>), AppError> {
    let token = s
        .notif_svc
        .register_device_token(body, claims.user_id)
        .await
        .map_err(|e| AppError::Validation(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(ApiResponse::ok(token))))
}

pub async fn list_device_tokens(
    State(s): State<AppState>,
    claims: axum::Extension<AuthClaims>,
) -> Result<Json<ApiResponse<Vec<DeviceTokenResponse>>>, AppError> {
    let tokens = s
        .notif_svc
        .list_device_tokens(claims.user_id)
        .await
        .map_err(AppError::Internal)?;

    Ok(Json(ApiResponse::ok(tokens)))
}

pub async fn delete_device_token(
    State(s): State<AppState>,
    claims: axum::Extension<AuthClaims>,
    Path(token): Path<String>,
) -> Result<StatusCode, AppError> {
    s.notif_svc
        .delete_device_token(&token, claims.user_id)
        .await
        .map_err(AppError::Internal)?;

    Ok(StatusCode::NO_CONTENT)
}
