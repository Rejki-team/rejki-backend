use super::AppState;
use crate::application::dto::{CreateIklanPekerjaanInput, IklanPekerjaanResponse, ListQuery};
use auth_service_client::AuthClaims;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Response,
    Json,
};
use common_errors::{created_response, ApiResponse, AppError, ValidatedJson};
use uuid::Uuid;

pub async fn list(
    State(s): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<ApiResponse<Vec<IklanPekerjaanResponse>>>, AppError> {
    Ok(Json(ApiResponse::ok(
        s.svc.list(q).await.map_err(AppError::Internal)?,
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
    claims: axum::Extension<AuthClaims>,
    ValidatedJson(body): ValidatedJson<CreateIklanPekerjaanInput>,
) -> Result<Response, AppError> {
    let item = s
        .svc
        .create(claims.user_id, body)
        .await
        .map_err(AppError::Internal)?;
    let id = item.id;
    Ok(created_response(
        ApiResponse::ok(item),
        &format!("/api/v1/pekerjaan/{id}"),
    ))
}

pub async fn delete_iklan(
    State(s): State<AppState>,
    claims: axum::Extension<AuthClaims>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let deleted = s
        .svc
        .delete(id, claims.user_id)
        .await
        .map_err(AppError::Internal)?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound("iklan tidak ditemukan".into()))
    }
}

/// Health check service (tanpa auth).
pub async fn health() -> (StatusCode, Json<serde_json::Value>) {
    (StatusCode::OK, Json(serde_json::json!({ "status": "ok" })))
}
