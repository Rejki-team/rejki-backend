//! Integration tests untuk RBAC iklan pelatihan.
//!
//! Naming: test_{unit}_given_{cond}_when_{action}_then_{expect}
//!
//! Jalankan:
//!   cargo test --test pelatihan_rbac_test -- --test-threads=1

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::json;
use tower::ServiceExt;

use common::{
    body_json, build_test_app, fixtures::clean_test_data, get_authed, post_authed, seed_admin,
    seed_user, test_pool,
};

async fn setup() -> (axum::Router, sqlx::PgPool) {
    let _ = dotenvy::from_filename(".env.test");
    let pool = test_pool().await;
    clean_test_data(&pool).await;
    let app = build_test_app(pool.clone()).await;
    (app, pool)
}

#[tokio::test]
async fn test_admin_create_given_valid_admin_when_create_then_201() {
    let (app, pool) = setup().await;
    let admin = seed_admin(&pool, "rbac1").await;

    let resp = app
        .oneshot(post_authed(
            "/api/v1/pelatihan/admin/pelatihan",
            &admin.access_token,
            json!({
                "judul": "Admin Training",
                "penyelenggara": "Rejki",
                "deskripsi": "Admin created",
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = body_json(resp).await;
    // Admin-created → langsung verifikasi_diterima
    assert_eq!(body["data"]["created_by_role"], "admin");
}

#[tokio::test]
async fn test_admin_create_given_user_when_create_then_403() {
    let (app, pool) = setup().await;
    let user = seed_user(&pool, "rbac2").await;

    let resp = app
        .oneshot(post_authed(
            "/api/v1/pelatihan/admin/pelatihan",
            &user.access_token,
            json!({
                "judul": "User Attempt",
                "penyelenggara": "Test",
                "deskripsi": "Should be forbidden",
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_admin_create_given_anon_when_create_then_401() {
    let (app, _pool) = setup().await;

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/pelatihan/admin/pelatihan")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "judul": "Anon Attempt",
                "penyelenggara": "Test",
                "deskripsi": "Should be unauthorized",
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_admin_list_given_admin_when_list_then_200() {
    let (app, pool) = setup().await;
    let admin = seed_admin(&pool, "rbac3").await;

    let resp = app
        .clone()
        .oneshot(get_authed(
            "/api/v1/pelatihan/admin/pelatihan",
            &admin.access_token,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}
