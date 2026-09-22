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
    seed_report_with_due_date, seed_user, test_pool,
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
async fn test_create_laporkan_iklan_given_valid_input_when_create_then_201() {
    let (app, pool) = setup().await;
    let user = seed_user(&pool, "crt1").await;
    let target_id = uuid::Uuid::now_v7();

    let resp = app
        .oneshot(post_authed(
            "/api/v1/reports/laporkan-iklan",
            &user.access_token,
            serde_json::json!({
                "target_type": "iklan",
                "target_id": target_id,
                "keterangan": "konten tidak pantas sama sekali dan melanggar aturan platform",
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = body_json(resp).await;
    assert!(body["success"].as_bool().unwrap());
    let data = &body["data"];
    assert!(!data["id"].as_str().unwrap().is_empty());
    assert_eq!(data["reporter_id"].as_str().unwrap(), user.id.to_string());
    assert_eq!(data["report_type"], "laporkan_iklan");
    assert_eq!(data["target_type"], "iklan");
    assert_eq!(data["target_id"].as_str().unwrap(), target_id.to_string());
    assert_eq!(data["status"], "pending");
    assert_eq!(data["is_overdue"], false);
    assert!(!data["due_date"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn test_create_laporkan_iklan_given_alasan_terlalu_pendek_when_create_then_422() {
    let (app, pool) = setup().await;
    let user = seed_user(&pool, "crt1b").await;

    let resp = app
        .oneshot(post_authed(
            "/api/v1/reports/laporkan-iklan",
            &user.access_token,
            serde_json::json!({
                "target_type": "iklan",
                "target_id": uuid::Uuid::now_v7(),
                "keterangan": "terlalu pendek",
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
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
            "/api/v1/reports/laporkan-iklan",
            "invalid-token",
            serde_json::json!({
                "target_type": "iklan",
                "target_id": uuid::Uuid::now_v7(),
                "keterangan": "konten tidak pantas sama sekali dan melanggar aturan platform",
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ── Overdue (P1.5) ────────────────────────────────────────────────────────

#[tokio::test]
async fn test_admin_get_given_due_date_lewat_when_get_then_is_overdue_true() {
    let (app, pool) = setup().await;
    let reporter = seed_user(&pool, "ovd1").await;
    let admin = seed_admin(&pool, "ovd1").await;
    // due_date 1 hari yang lalu, status masih pending → melewati SLA.
    let past_due = chrono::Utc::now() - chrono::Duration::days(1);
    let report_id = seed_report_with_due_date(&pool, reporter.id, "ovd1", past_due).await;

    let resp = app
        .oneshot(get_authed(
            &format!("/api/v1/reports/admin/{report_id}"),
            &admin.access_token,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_json(resp).await;
    assert_eq!(body["data"]["is_overdue"], true);
}

#[tokio::test]
async fn test_admin_get_given_due_date_belum_lewat_when_get_then_is_overdue_false() {
    let (app, pool) = setup().await;
    let reporter = seed_user(&pool, "ovd2").await;
    let admin = seed_admin(&pool, "ovd2").await;
    let future_due = chrono::Utc::now() + chrono::Duration::days(3);
    let report_id = seed_report_with_due_date(&pool, reporter.id, "ovd2", future_due).await;

    let resp = app
        .oneshot(get_authed(
            &format!("/api/v1/reports/admin/{report_id}"),
            &admin.access_token,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_json(resp).await;
    assert_eq!(body["data"]["is_overdue"], false);
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
