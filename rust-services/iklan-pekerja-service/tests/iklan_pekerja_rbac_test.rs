//! Integration tests untuk RBAC iklan pekerja.
//!
//! Naming: test_{unit}_given_{cond}_when_{action}_then_{expect}

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use common::{
    build_test_app, fixtures::clean_test_data, fixtures::seed_admin, fixtures::seed_user,
    get_authed, test_pool,
};

async fn setup() -> (axum::Router, sqlx::PgPool) {
    let _ = dotenvy::from_filename(".env.test");
    let pool = test_pool().await;
    clean_test_data(&pool).await;
    let app = build_test_app(pool.clone()).await;
    (app, pool)
}

#[tokio::test]
async fn test_create_given_anon_when_create_then_401() {
    let (app, _pool) = setup().await;

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/pekerja")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({"nama": "Anon", "keahlian": ["test"], "deskripsi": "test"})
                .to_string(),
        ))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_admin_list_given_user_when_list_then_403() {
    let (app, pool) = setup().await;
    let user = seed_user(&pool, "rbac1").await;

    let resp = app
        .oneshot(get_authed("/api/v1/pekerja/admin", &user.access_token))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_admin_list_given_admin_when_list_then_200() {
    let (app, pool) = setup().await;
    let admin = seed_admin(&pool, "rbac2").await;

    let resp = app
        .oneshot(get_authed("/api/v1/pekerja/admin", &admin.access_token))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}
