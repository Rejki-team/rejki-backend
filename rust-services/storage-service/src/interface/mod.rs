use axum::{routing::post, Json, Router};

use common_errors::ApiResponse;

pub fn router() -> Router {
    Router::new().route(
        "/upload",
        post(|| async {
            Json(ApiResponse::ok(
                "storage endpoint — presigned URL via StorageClient trait",
            ))
        }),
    )
}
