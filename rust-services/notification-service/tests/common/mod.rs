#![allow(dead_code)]
use std::sync::Arc;

use axum::{body::Body, http::Request, Router};
use serde_json::Value;
use sqlx::PgPool;

pub mod fixtures;

#[allow(unused_imports)]
pub use fixtures::{clean_test_data, seed_notification, seed_user, TestUser};

/// Test app — build router untuk notification-service saja dengan pool test.
/// Router ditempatkan di bawah `/api/v1/notif`.
pub async fn build_test_app(pool: PgPool) -> Router {
    let private_pem = std::fs::read_to_string(
        std::env::var("JWT_PRIVATE_KEY_PATH").unwrap_or_else(|_| "./keys/private.pem".into()),
    )
    .expect("JWT_PRIVATE_KEY_PATH not found");

    let public_pem = std::fs::read_to_string(
        std::env::var("JWT_PUBLIC_KEY_PATH").unwrap_or_else(|_| "./keys/public.pem".into()),
    )
    .expect("JWT_PUBLIC_KEY_PATH not found");

    let jwt = Arc::new(
        auth_service::JwtService::from_files(&private_pem, &public_pem, 900)
            .expect("JwtService init failed"),
    );

    let auth_repo = Arc::new(auth_service::PgAuthRepository::new(pool.clone()));
    let auth_client: Arc<dyn auth_service::AuthClient> =
        Arc::new(auth_service::AuthInProcessClient::new(jwt, auth_repo));

    let notif_router = notification_service::router(pool, auth_client, None);

    Router::new().nest("/api/v1/notif", notif_router)
}

/// Koneksi ke database test.
pub async fn test_pool() -> PgPool {
    let _ = dotenvy::from_filename(".env.test");

    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL harus di-set di .env.test");

    sqlx::PgPool::connect(&db_url)
        .await
        .expect("gagal connect ke test database")
}

/// Helper: POST request dengan JSON body dan Bearer token.
pub fn post_authed(uri: &str, token: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

/// Helper: GET request dengan Bearer token.
pub fn get_authed(uri: &str, token: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(uri)
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap()
}

/// Helper: GET request tanpa auth.
pub fn get_anon(uri: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

/// Helper: PATCH request dengan Bearer token.
pub fn patch_authed(uri: &str, token: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method("PATCH")
        .uri(uri)
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

/// Helper: PATCH request tanpa body dengan Bearer token.
pub fn patch_authed_empty(uri: &str, token: &str) -> Request<Body> {
    Request::builder()
        .method("PATCH")
        .uri(uri)
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap()
}

/// Helper: DELETE request dengan Bearer token.
pub fn delete_authed(uri: &str, token: &str) -> Request<Body> {
    Request::builder()
        .method("DELETE")
        .uri(uri)
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap()
}

/// Helper: Baca response body sebagai JSON.
pub async fn body_json(resp: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
}
