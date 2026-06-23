//! Integration tests untuk RBAC artikel corporate comms.
//!
//! Semua endpoint adalah admin-only (require_role(60)):
//!   POST   /api/v1/articles/       → buat
//!   GET    /api/v1/articles/       → daftar
//!   GET    /api/v1/articles/{id}   → detail
//!   PATCH  /api/v1/articles/{id}   → ubah
//!   DELETE /api/v1/articles/{id}   → hapus
//!
//! Tiga level akses:
//!   - Anonim  (no token)   → 401 UNAUTHORIZED
//!   - User    (rank 20)    → 403 FORBIDDEN
//!   - Admin   (rank >= 60) → 200 / 201 / 204
//!
//! Jalankan:
//!   cargo test --test article_rbac_test -- --test-threads=1

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::json;
use tower::ServiceExt;

use common::{
    build_test_app, delete_authed, fixtures::clean_test_data, get_authed, patch_authed,
    post_authed, seed_admin, seed_article, seed_user, test_pool,
};

async fn setup() -> (axum::Router, sqlx::PgPool) {
    let _ = dotenvy::from_filename(".env.test");
    let pool = test_pool().await;
    clean_test_data(&pool).await;
    let app = build_test_app(pool.clone()).await;
    (app, pool)
}

// ── HELPERS ───────────────────────────────────────────────────────────────────

/// Raw request helper (no auth header) for testing 401.
fn raw_req(method: &str, uri: &str, body: Option<serde_json::Value>) -> Request<Body> {
    let mut builder = Request::builder().method(method).uri(uri);
    if body.is_some() {
        builder = builder.header("content-type", "application/json");
    }
    let b = body.map(|v| v.to_string()).unwrap_or_default();
    builder.body(Body::from(b)).unwrap()
}

fn anon_post(uri: &str, body: serde_json::Value) -> Request<Body> {
    raw_req("POST", uri, Some(body))
}
fn anon_get(uri: &str) -> Request<Body> {
    raw_req("GET", uri, None)
}
fn anon_patch(uri: &str, body: serde_json::Value) -> Request<Body> {
    raw_req("PATCH", uri, Some(body))
}
fn anon_delete(uri: &str) -> Request<Body> {
    raw_req("DELETE", uri, None)
}

fn sample_create() -> serde_json::Value {
    json!({"title": "RBAC Test Article","body": "Body content here"})
}

fn sample_update() -> serde_json::Value {
    json!({"title": "Updated Title","body": "Updated body","category":"informasi","photo_object_key":null})
}

const LIST_URI: &str = "/api/v1/articles";

// ═══════════════════════════════════════════════════════════════════════════════
// CREATE — POST /api/v1/articles/
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_create_given_anon_when_create_then_401() {
    let (app, _pool) = setup().await;
    let resp = app
        .oneshot(anon_post(LIST_URI, sample_create()))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_create_given_user_when_create_then_403() {
    let (app, pool) = setup().await;
    let user = seed_user(&pool, "rbac-crt-u").await;
    let resp = app
        .oneshot(post_authed(LIST_URI, &user.access_token, sample_create()))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_create_given_admin_when_create_then_201() {
    let (app, pool) = setup().await;
    let admin = seed_admin(&pool, "rbac-crt-a").await;
    let resp = app
        .oneshot(post_authed(LIST_URI, &admin.access_token, sample_create()))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
}

// ═══════════════════════════════════════════════════════════════════════════════
// LIST — GET /api/v1/articles/
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_list_given_anon_when_list_then_401() {
    let (app, _pool) = setup().await;
    let resp = app.oneshot(anon_get(LIST_URI)).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_list_given_user_when_list_then_403() {
    let (app, pool) = setup().await;
    let user = seed_user(&pool, "rbac-lst-u").await;
    let resp = app
        .oneshot(get_authed(LIST_URI, &user.access_token))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_list_given_admin_when_list_then_200() {
    let (app, pool) = setup().await;
    let admin = seed_admin(&pool, "rbac-lst-a").await;
    let resp = app
        .oneshot(get_authed(LIST_URI, &admin.access_token))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

// ═══════════════════════════════════════════════════════════════════════════════
// GET BY ID — GET /api/v1/articles/{id}
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_get_given_anon_when_get_then_401() {
    let (app, pool) = setup().await;
    let admin = seed_admin(&pool, "rbac-get-a1").await;
    let article_id = seed_article(&pool, admin.id, "rbac-get-a1").await;
    let resp = app
        .oneshot(anon_get(&format!("/api/v1/articles/{article_id}")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_get_given_user_when_get_then_403() {
    let (app, pool) = setup().await;
    let admin = seed_admin(&pool, "rbac-get-a2").await;
    let user = seed_user(&pool, "rbac-get-u").await;
    let article_id = seed_article(&pool, admin.id, "rbac-get-a2").await;
    let resp = app
        .oneshot(get_authed(
            &format!("/api/v1/articles/{article_id}"),
            &user.access_token,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_get_given_admin_when_get_then_200() {
    let (app, pool) = setup().await;
    let admin = seed_admin(&pool, "rbac-get-a3").await;
    let article_id = seed_article(&pool, admin.id, "rbac-get-a3").await;
    let resp = app
        .oneshot(get_authed(
            &format!("/api/v1/articles/{article_id}"),
            &admin.access_token,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

// ═══════════════════════════════════════════════════════════════════════════════
// UPDATE — PATCH /api/v1/articles/{id}
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_update_given_anon_when_update_then_401() {
    let (app, pool) = setup().await;
    let admin = seed_admin(&pool, "rbac-upd-a1").await;
    let article_id = seed_article(&pool, admin.id, "rbac-upd-a1").await;
    let resp = app
        .oneshot(anon_patch(
            &format!("/api/v1/articles/{article_id}"),
            sample_update(),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_update_given_user_when_update_then_403() {
    let (app, pool) = setup().await;
    let admin = seed_admin(&pool, "rbac-upd-a2").await;
    let user = seed_user(&pool, "rbac-upd-u").await;
    let article_id = seed_article(&pool, admin.id, "rbac-upd-a2").await;
    let resp = app
        .oneshot(patch_authed(
            &format!("/api/v1/articles/{article_id}"),
            &user.access_token,
            sample_update(),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_update_given_admin_when_update_then_200() {
    let (app, pool) = setup().await;
    let admin = seed_admin(&pool, "rbac-upd-a3").await;
    let article_id = seed_article(&pool, admin.id, "rbac-upd-a3").await;
    let resp = app
        .oneshot(patch_authed(
            &format!("/api/v1/articles/{article_id}"),
            &admin.access_token,
            sample_update(),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

// ═══════════════════════════════════════════════════════════════════════════════
// DELETE — DELETE /api/v1/articles/{id}
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_delete_given_anon_when_delete_then_401() {
    let (app, pool) = setup().await;
    let admin = seed_admin(&pool, "rbac-del-a1").await;
    let article_id = seed_article(&pool, admin.id, "rbac-del-a1").await;
    let resp = app
        .oneshot(anon_delete(&format!("/api/v1/articles/{article_id}")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_delete_given_user_when_delete_then_403() {
    let (app, pool) = setup().await;
    let admin = seed_admin(&pool, "rbac-del-a2").await;
    let user = seed_user(&pool, "rbac-del-u").await;
    let article_id = seed_article(&pool, admin.id, "rbac-del-a2").await;
    let resp = app
        .oneshot(delete_authed(
            &format!("/api/v1/articles/{article_id}"),
            &user.access_token,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_delete_given_admin_when_delete_then_204() {
    let (app, pool) = setup().await;
    let admin = seed_admin(&pool, "rbac-del-a3").await;
    let article_id = seed_article(&pool, admin.id, "rbac-del-a3").await;
    let resp = app
        .oneshot(delete_authed(
            &format!("/api/v1/articles/{article_id}"),
            &admin.access_token,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}
