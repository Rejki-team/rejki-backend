//! Integration tests untuk report-service — report flow (user create, admin list/detail).
//!
//! Naming: test_{unit}_given_{cond}_when_{action}_then_{expect}
//!
//! Jalankan:
//!   cargo test --test report_flow_test -- --test-threads=1
//!
//! Prerequisite:
//!   - PostgreSQL test DB hidup (DATABASE_URL dari .env.test)
//!   - Migrasi sudah dijalankan
//!   - RSA key pair ada di ./keys/

mod common;

use axum::http::StatusCode;
use common::{
    body_json, build_test_app, clean_test_data, get_authed, post_authed, seed_admin, seed_report,
    seed_user, test_pool,
};
use tower::ServiceExt;

async fn setup() -> (axum::Router, sqlx::PgPool) {
    let _ = dotenvy::from_filename(".env.test");
    let pool = test_pool().await;
    clean_test_data(&pool).await;
    let app = build_test_app(pool.clone()).await;
    (app, pool)
}

// ── Create report ─────────────────────────────────────────────────────────

#[tokio::test]
async fn test_create_report_given_valid_input_when_create_then_201() {
    let (app, pool) = setup().await;
    let user = seed_user(&pool, "crt1").await;
    let target_id = uuid::Uuid::now_v7();

    let resp = app
        .oneshot(post_authed(
            "/api/v1/reports",
            &user.access_token,
            serde_json::json!({
                "target_type": "iklan",
                "target_id": target_id,
                "keterangan": "konten tidak pantas",
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = body_json(resp).await;
    assert!(body["success"].as_bool().unwrap());
    let data = &body["data"];
    assert!(data["id"].as_str().unwrap().len() > 0);
    assert_eq!(data["reporter_id"].as_str().unwrap(), user.id.to_string());
    assert_eq!(data["target_type"], "iklan");
    assert_eq!(data["target_id"].as_str().unwrap(), target_id.to_string());
    assert_eq!(data["keterangan"], "konten tidak pantas");
    assert_eq!(data["status"], "pending");
}

// ── User cannot access admin list ─────────────────────────────────────────

#[tokio::test]
async fn test_admin_list_given_user_token_when_list_then_403() {
    let (app, pool) = setup().await;
    let user = seed_user(&pool, "lst1").await;

    let resp = app
        .oneshot(get_authed("/api/v1/reports/admin", &user.access_token))
        .await
        .unwrap();
    // User tidak punya role moderator (60) → 403 INSUFFICIENT_ROLE
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

// ── Admin list reports ────────────────────────────────────────────────────

#[tokio::test]
async fn test_admin_list_given_admin_token_when_list_then_200() {
    let (app, pool) = setup().await;
    let user = seed_user(&pool, "lst2").await;
    let admin = seed_admin(&pool, "lst2").await;

    // Seed a report so list is non-empty
    seed_report(&pool, user.id, "lst2").await;

    let resp = app
        .oneshot(get_authed("/api/v1/reports/admin", &admin.access_token))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_json(resp).await;
    assert!(body["success"].as_bool().unwrap());
    let items = body["data"].as_array().expect("data harus berupa array");
    assert!(!items.is_empty(), "seharusnya ada minimal 1 report");
}

// ── Anonymous cannot create report ────────────────────────────────────────

#[tokio::test]
async fn test_create_report_given_anon_when_create_then_401() {
    let (app, _pool) = setup().await;

    let resp = app
        .oneshot(post_authed(
            "/api/v1/reports",
            "invalid-token",
            serde_json::json!({
                "target_type": "iklan",
                "target_id": uuid::Uuid::now_v7(),
                "keterangan": "konten tidak pantas",
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ── Admin get nonexistent report ─────────────────────────────────────────

#[tokio::test]
async fn test_admin_get_given_nonexistent_id_when_get_then_404() {
    let (app, pool) = setup().await;
    let admin = seed_admin(&pool, "get1").await;
    let fake_id = uuid::Uuid::now_v7();

    let resp = app
        .oneshot(get_authed(
            &format!("/api/v1/reports/admin/{fake_id}"),
            &admin.access_token,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
