use axum::{
    extract::{Query, State},
    response::Json,
};
use common_errors::{ApiResponse, AppError};

use super::AppState;
use crate::application::dto::{InsightsQuery, RefreshResponse};

pub async fn get_user_stats(
    State(s): State<AppState>,
) -> Result<Json<ApiResponse<crate::application::dto::UserStatsResponse>>, AppError> {
    let data = s.svc.get_user_stats().await.map_err(AppError::Internal)?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn get_iklan_stats(
    State(s): State<AppState>,
    Query(q): Query<InsightsQuery>,
) -> Result<Json<ApiResponse<Vec<crate::application::dto::IklanStatsResponse>>>, AppError> {
    let data = s
        .svc
        .get_iklan_stats(q.vertikal.as_deref())
        .await
        .map_err(AppError::Internal)?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn get_geo_stats(
    State(s): State<AppState>,
    Query(q): Query<InsightsQuery>,
) -> Result<Json<ApiResponse<Vec<crate::application::dto::GeoStatsResponse>>>, AppError> {
    let data = s
        .svc
        .get_geo_stats(q.province_id.as_deref())
        .await
        .map_err(AppError::Internal)?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn get_engagement_stats(
    State(s): State<AppState>,
) -> Result<Json<ApiResponse<crate::application::dto::EngagementStatsResponse>>, AppError> {
    let data = s
        .svc
        .get_engagement_stats()
        .await
        .map_err(AppError::Internal)?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn get_canvassing(
    State(s): State<AppState>,
) -> Result<Json<ApiResponse<crate::application::dto::CanvassingResponse>>, AppError> {
    let data = s.svc.get_canvassing().await.map_err(AppError::Internal)?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn refresh(
    State(s): State<AppState>,
) -> Result<Json<ApiResponse<RefreshResponse>>, AppError> {
    let data = s.svc.refresh().await.map_err(AppError::Internal)?;
    Ok(Json(ApiResponse::ok(data)))
}
