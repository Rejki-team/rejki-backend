use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};

use common_errors::ApiResponse;

use super::{AppState, ParentQuery};

pub async fn provinces(
    State(s): State<AppState>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), common_errors::AppError> {
    let list = s
        .region_svc
        .list_provinces()
        .await
        .map_err(common_errors::AppError::Internal)?;
    Ok((
        StatusCode::OK,
        Json(ApiResponse::ok(
            serde_json::to_value(list).unwrap_or_default(),
        )),
    ))
}

pub async fn regencies(
    State(s): State<AppState>,
    Query(q): Query<ParentQuery>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), common_errors::AppError> {
    let pid = q.province_id.as_deref().unwrap_or("");
    let list = s
        .region_svc
        .list_regencies(pid)
        .await
        .map_err(common_errors::AppError::Internal)?;
    Ok((
        StatusCode::OK,
        Json(ApiResponse::ok(
            serde_json::to_value(list).unwrap_or_default(),
        )),
    ))
}

pub async fn districts(
    State(s): State<AppState>,
    Query(q): Query<ParentQuery>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), common_errors::AppError> {
    let pid = q.regency_id.as_deref().unwrap_or("");
    let list = s
        .region_svc
        .list_districts(pid)
        .await
        .map_err(common_errors::AppError::Internal)?;
    Ok((
        StatusCode::OK,
        Json(ApiResponse::ok(
            serde_json::to_value(list).unwrap_or_default(),
        )),
    ))
}

pub async fn villages(
    State(s): State<AppState>,
    Query(q): Query<ParentQuery>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), common_errors::AppError> {
    let pid = q.district_id.as_deref().unwrap_or("");
    let list = s
        .region_svc
        .list_villages(pid)
        .await
        .map_err(common_errors::AppError::Internal)?;
    Ok((
        StatusCode::OK,
        Json(ApiResponse::ok(
            serde_json::to_value(list).unwrap_or_default(),
        )),
    ))
}
