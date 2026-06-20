//! Integration tests untuk report-service — admin review flow.
//!
//! Naming: test_{unit}_given_{cond}_when_{action}_then_{expect}
//!
//! Jalankan:
//!   cargo test --test report_review_test -- --test-threads=1
//!
//! Prerequisite:
//!   - PostgreSQL test DB hidup (DATABASE_URL dari .env.test)
//!   - Migrasi sudah dijalankan
//!   - RSA key pair ada di ./keys/

mod common;

use axum::http::StatusCode;
use common::{
    body_json, build_test_app, clean_test_data, post_authed, seed_admin, seed_report, seed_user,
    test_pool,
};
use tower::ServiceExt;

async fn setup() -> (axum::Router, sqlx::PgPool) {
    let _ = dotenvy::from_filename(".env.test");
    let pool = test_pool().await;
    clean_test_data(&pool).await;
    let app = build_test_app(pool.clone()).await;
    (app, pool)
}

// ── Admin approve report ─────────────────────────────────────────────────

#[tokio::test]
async fn test_review_given_pending_report_when_approve_then_200() {
    let (app, pool) = setup().await;
    let reporter = seed_user(&pool, "rvw1a").await;
    let admin = seed_admin(&pool, "rvw1a").await;
    let report_id = seed_report(&pool, reporter.id, "rvw1a").await;

    let resp = app
        .clone()
        .oneshot(post_authed(
            &format!("/api/v1/reports/admin/{report_id}/review"),
            &admin.access_token,
            serde_json::json!({
                "approved": true,
                "action_note": "ok",
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_json(resp).await;
    assert!(body["success"].as_bool().unwrap());
    assert_eq!(body["data"]["status"], "resolved");
}

// ── Admin reject report ──────────────────────────────────────────────────

#[tokio::test]
async fn test_review_given_pending_report_when_reject_then_200() {
    let (app, pool) = setup().await;
    let reporter = seed_user(&pool, "rvw2a").await;
    let admin = seed_admin(&pool, "rvw2a").await;
    let report_id = seed_report(&pool, reporter.id, "rvw2a").await;

    let resp = app
        .clone()
        .oneshot(post_authed(
            &format!("/api/v1/reports/admin/{report_id}/review"),
            &admin.access_token,
            serde_json::json!({
                "approved": false,
                "action_note": "tidak memenuhi syarat",
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_json(resp).await;
    assert!(body["success"].as_bool().unwrap());
    assert_eq!(body["data"]["status"], "rejected");
}

// ── Re-review terminal report → 409 ─────────────────────────────────────

#[tokio::test]
async fn test_review_given_terminal_report_when_review_again_then_409() {
    let (app, pool) = setup().await;
    let reporter = seed_user(&pool, "rvw3a").await;
    let admin = seed_admin(&pool, "rvw3a").await;
    let report_id = seed_report(&pool, reporter.id, "rvw3a").await;

    // First review (approve) → OK
    let resp = app
        .clone()
        .oneshot(post_authed(
            &format!("/api/v1/reports/admin/{report_id}/review"),
            &admin.access_token,
            serde_json::json!({
                "approved": true,
                "action_note": "ok",
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Second review → 409 because status is now terminal
    let resp = app
        .oneshot(post_authed(
            &format!("/api/v1/reports/admin/{report_id}/review"),
            &admin.access_token,
            serde_json::json!({
                "approved": false,
                "action_note": "mencoba lagi",
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}
