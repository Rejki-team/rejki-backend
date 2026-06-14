use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use auth_service_client::AuthClaims;
use common_errors::{ApiResponse, AppError, ValidatedJson};

use super::AppState;
use crate::application::dto::{NotificationResponse, SendNotificationInput};

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
    s.notif_svc.send(body).await.map_err(AppError::Internal)?;

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
