use super::AppState;
use crate::application::dto::{CreateRatingInput, RatingAggregateResponse};
use auth_service_client::AuthClaims;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Response,
    Extension, Json,
};
use common_errors::{created_response, ApiResponse, AppError, ValidatedJson};
use uuid::Uuid;

/// PRD §5.15 — "Kirim Rating".
pub async fn create_rating(
    State(s): State<AppState>,
    Extension(claims): Extension<AuthClaims>,
    ValidatedJson(body): ValidatedJson<CreateRatingInput>,
) -> Result<Response, AppError> {
    let item = s
        .svc
        .create_rating(claims.user_id, body)
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("sudah memberikan") {
                AppError::Conflict(msg)
            } else if msg.contains("belum berstatus Selesai") {
                AppError::Forbidden(msg)
            } else {
                AppError::Validation(msg)
            }
        })?;
    let id = item.id;
    Ok(created_response(
        ApiResponse::ok(item),
        &format!("/api/v1/rating/{id}"),
    ))
}

/// P5.4 — rating keaktifan gabungan seorang user.
pub async fn get_aggregate(
    State(s): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<ApiResponse<RatingAggregateResponse>>, AppError> {
    Ok(Json(ApiResponse::ok(
        s.svc
            .get_aggregate(user_id)
            .await
            .map_err(AppError::Internal)?,
    )))
}

pub async fn health() -> (StatusCode, Json<serde_json::Value>) {
    (StatusCode::OK, Json(serde_json::json!({ "status": "ok" })))
}
