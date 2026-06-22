//! Integration tests untuk insights-service — CEO Analytics Endpoints (W3D-12).
//!
//! Menggunakan Podman PostgreSQL (container rejki-pg-dev, DB rejki_db).
//! Setup sudah dijalankan: schema + seed data + MV migration.
//!
//! Jalankan:
//!   cargo test -p insights-service --test insights_integration_test -- --test-threads=1
//!
//! Prerequisite:
//!   - Podman machine running: podman machine start podman-machine-default
//!   - PG container: podman start rejki-pg-dev (rootful mode)
//!   - RSA key pair di ./keys/
//!   - .env.test dengan DATABASE_URL

mod common;

use axum::http::StatusCode;
use common::{
    body_json, build_test_app, clean_test_data, executive_token, get_anon, get_authed, post_authed,
    super_admin_token, test_pool, user_token,
};
use tower::ServiceExt;

async fn setup() -> (axum::Router, sqlx::PgPool) {
    let _ = dotenvy::from_filename(".env.test");
    let pool = test_pool().await;
    clean_test_data(&pool).await;
    let app = build_test_app(pool.clone()).await;
    (app, pool)
}

// ── User Stats ──────────────────────────────────────────────────────────

#[tokio::test]
async fn test_user_stats_given_executive_when_get_then_200() {
    let (app, pool) = setup().await;
    let token = executive_token(&pool, "usr1").await;

    let resp = app
        .oneshot(get_authed("/insights/users", &token))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_json(resp).await;
    assert!(body["success"].as_bool().unwrap_or(false));
    let data = &body["data"];
    assert!(
        data["total_users"].is_i64(),
        "total_users should be integer"
    );
    assert!(data["conversion_funnel"]["registered"].is_i64());
}

// ── Iklan Stats ─────────────────────────────────────────────────────────

#[tokio::test]
async fn test_iklan_stats_given_executive_when_get_then_200() {
    let (app, pool) = setup().await;
    let token = executive_token(&pool, "ik1").await;

    let resp = app
        .oneshot(get_authed("/insights/iklan", &token))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_json(resp).await;
    assert!(body["success"].as_bool().unwrap_or(false));
    let data = body["data"].as_array().expect("data harus array");
    assert_eq!(data.len(), 4, "harus 4 vertikal");
}

#[tokio::test]
async fn test_iklan_stats_given_executive_when_filter_vertikal_then_filtered() {
    let (app, pool) = setup().await;
    let token = executive_token(&pool, "ik2").await;

    let resp = app
        .oneshot(get_authed("/insights/iklan?vertikal=pekerjaan", &token))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_json(resp).await;
    let data = body["data"].as_array().expect("data harus array");
    assert_eq!(data.len(), 1, "hanya 1 vertikal");
    assert_eq!(data[0]["vertikal"], "pekerjaan");
}

// ── Geo Stats ───────────────────────────────────────────────────────────

#[tokio::test]
async fn test_geo_stats_given_executive_when_get_then_200() {
    let (app, pool) = setup().await;
    let token = executive_token(&pool, "geo1").await;

    let resp = app
        .oneshot(get_authed("/insights/geo", &token))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_json(resp).await;
    assert!(body["success"].as_bool().unwrap_or(false));
    let data = body["data"].as_array().expect("data harus array");
    assert!(!data.is_empty(), "minimal 1 provinsi");

    let jakarta = data.iter().find(|p| p["province_id"] == "31");
    assert!(jakarta.is_some(), "DKI Jakarta harus ada");
    assert!(
        jakarta.unwrap()["canvassing_score"].as_i64().unwrap_or(0) > 0,
        "Jakarta harus punya canvassing_score > 0"
    );
    assert!(
        jakarta.unwrap()["priority_tier"].as_str().is_some(),
        "harus punya priority_tier"
    );
}

#[tokio::test]
async fn test_geo_stats_given_executive_when_filter_province_then_filtered() {
    let (app, pool) = setup().await;
    let token = executive_token(&pool, "geo2").await;

    let resp = app
        .oneshot(get_authed("/insights/geo?province_id=31", &token))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_json(resp).await;
    let data = body["data"].as_array().expect("data harus array");
    assert_eq!(data.len(), 1, "hanya DKI Jakarta");
    assert_eq!(data[0]["province_id"], "31");
}

// ── Engagement Stats ────────────────────────────────────────────────────

#[tokio::test]
async fn test_engagement_stats_given_executive_when_get_then_200() {
    let (app, pool) = setup().await;
    let token = executive_token(&pool, "eng1").await;

    let resp = app
        .oneshot(get_authed("/insights/engagement", &token))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_json(resp).await;
    assert!(body["success"].as_bool().unwrap_or(false));
    let data = &body["data"];
    assert!(data["messages_7d"].is_i64());
    assert!(data["messages_30d"].is_i64());
}

// ── Canvassing ──────────────────────────────────────────────────────────

#[tokio::test]
async fn test_canvassing_given_executive_when_get_then_200() {
    let (app, pool) = setup().await;
    let token = executive_token(&pool, "can1").await;

    let resp = app
        .oneshot(get_authed("/insights/canvassing", &token))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_json(resp).await;
    assert!(body["success"].as_bool().unwrap_or(false));

    let provinces = body["data"]["provinces"]
        .as_array()
        .expect("provinces array");
    assert!(!provinces.is_empty());

    let summary = &body["data"]["summary"];
    assert!(summary["total_provinces_analyzed"].as_i64().unwrap_or(0) > 0);
    assert!(summary["avg_canvassing_score"].as_f64().is_some());
}

// ── Refresh ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_refresh_given_executive_when_post_then_200() {
    let (app, pool) = setup().await;
    let token = executive_token(&pool, "ref1").await;

    let resp = app
        .oneshot(post_authed(
            "/insights/refresh",
            &token,
            serde_json::json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_json(resp).await;
    assert!(body["success"].as_bool().unwrap_or(false));
    assert_eq!(body["data"]["status"], "ok");
}

// ── RBAC: User cannot access ────────────────────────────────────────────

#[tokio::test]
async fn test_user_stats_given_user_role_when_get_then_403() {
    let (app, pool) = setup().await;
    let token = user_token(&pool, "rba1").await;

    let resp = app
        .oneshot(get_authed("/insights/users", &token))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

// ── RBAC: Super admin can access ───────────────────────────────────────

#[tokio::test]
async fn test_user_stats_given_super_admin_when_get_then_200() {
    let (app, pool) = setup().await;
    let token = super_admin_token(&pool, "adm1").await;

    let resp = app
        .oneshot(get_authed("/insights/users", &token))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

// ── Anonymous cannot access ─────────────────────────────────────────────

#[tokio::test]
async fn test_user_stats_given_anon_when_get_then_401() {
    let (app, _pool) = setup().await;

    let resp = app.oneshot(get_anon("/insights/users")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_refresh_given_anon_when_post_then_401() {
    let (app, _pool) = setup().await;

    let resp = app
        .oneshot(post_authed(
            "/insights/refresh",
            "invalid-token",
            serde_json::json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
