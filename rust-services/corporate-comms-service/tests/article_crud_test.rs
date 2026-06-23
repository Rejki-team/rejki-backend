//! Integration tests untuk CRUD dasar artikel corporate comms.
//!
//! Naming: test_{unit}_given_{cond}_when_{action}_then_{expect}
//!
//! Jalankan:
//!   cargo test --test article_crud_test -- --test-threads=1
//!
//! Prerequisite:
//!   - PostgreSQL test DB hidup (DATABASE_URL dari .env.test)
//!   - Migrasi sudah dijalankan
//!   - RSA key pair ada di ./keys/

mod common;

use axum::http::StatusCode;
use axum::Router;
use sqlx::PgPool;
use tower::ServiceExt;

use common::{
    body_json, build_test_app, delete_authed, fixtures::clean_test_data, get_authed, patch_authed,
    post_authed, seed_admin, seed_article, test_pool,
};

async fn setup() -> (Router, PgPool) {
    let _ = dotenvy::from_filename(".env.test");
    let pool = test_pool().await;
    clean_test_data(&pool).await;
    let app = build_test_app(pool.clone()).await;
    (app, pool)
}

// ── Create ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_create_given_admin_when_create_then_201() {
    let (app, pool) = setup().await;
    let admin = seed_admin(&pool, "crt1").await;

    let resp = app
        .oneshot(post_authed(
            "/api/v1/articles",
            &admin.access_token,
            serde_json::json!({
                "title": "Artikel Baru",
                "body": "Ini adalah isi artikel baru",
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Verify response has data
    let body = body_json(resp).await;
    assert!(
        body["data"].is_object() || body["data"].is_string(),
        "expected data in response"
    );
}

// ── List ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_list_given_seeded_article_when_list_then_200() {
    let (app, pool) = setup().await;
    let admin = seed_admin(&pool, "lst1").await;

    // Seed 2 artikel milik admin
    seed_article(&pool, admin.id, "lst_a").await;
    seed_article(&pool, admin.id, "lst_b").await;

    let resp = app
        .clone()
        .oneshot(get_authed("/api/v1/articles", &admin.access_token))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_json(resp).await;
    let items = body["data"].as_array().unwrap();
    // Harusnya ada minimal 2 item (data test lain bisa ikut)
    assert!(
        items.len() >= 2,
        "list harus >= 2 item, got {}",
        items.len()
    );

    // Verifikasi struktur item
    let titles: Vec<&str> = items.iter().map(|i| i["title"].as_str().unwrap()).collect();
    assert!(
        titles.iter().any(|j| j.contains("lst_a")),
        "lst_a harus ada di list"
    );
    assert!(
        titles.iter().any(|j| j.contains("lst_b")),
        "lst_b harus ada di list"
    );
}

// ── Get by ID ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_get_given_valid_id_when_get_then_200() {
    let (app, pool) = setup().await;
    let admin = seed_admin(&pool, "get1").await;

    // Seed an article for this admin
    let article_id = seed_article(&pool, admin.id, "get1").await;

    let resp = app
        .oneshot(get_authed(
            &format!("/api/v1/articles/{article_id}"),
            &admin.access_token,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_json(resp).await;
    let data = &body["data"];
    assert_eq!(data["id"].as_str().unwrap(), article_id.to_string());
    assert_eq!(data["author_id"].as_str().unwrap(), admin.id.to_string());
    assert!(data["title"].as_str().unwrap().contains("get1"));
}

#[tokio::test]
async fn test_get_given_nonexistent_id_when_get_then_404() {
    let (app, pool) = setup().await;
    let admin = seed_admin(&pool, "get2").await;
    let fake_id = uuid::Uuid::now_v7();

    let resp = app
        .oneshot(get_authed(
            &format!("/api/v1/articles/{fake_id}"),
            &admin.access_token,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ── Update ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_update_given_admin_when_update_then_200() {
    let (app, pool) = setup().await;
    let admin = seed_admin(&pool, "upd1").await;

    let article_id = seed_article(&pool, admin.id, "upd1").await;

    let resp = app
        .oneshot(patch_authed(
            &format!("/api/v1/articles/{article_id}"),
            &admin.access_token,
            serde_json::json!({
                "title": "Judul Diperbarui",
                "body": "Isi artikel yang sudah diperbarui",
                "category": "informasi",
                "photo_object_key": null,
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_json(resp).await;
    let data = &body["data"];
    assert!(data["title"].as_str().unwrap().contains("Diperbarui"));
    assert!(data["body"].as_str().unwrap().contains("artikel"));
    assert!(data["updated_at"].as_str().unwrap().len() > 0);
}

// ── Delete ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_delete_given_admin_when_delete_then_204() {
    let (app, pool) = setup().await;
    let admin = seed_admin(&pool, "del1").await;

    let article_id = seed_article(&pool, admin.id, "del1").await;

    let resp = app
        .clone()
        .oneshot(delete_authed(
            &format!("/api/v1/articles/{article_id}"),
            &admin.access_token,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // GET after soft-delete: behavior depends on service implementation
    // (some services filter deleted_at, some don't) — just verify delete succeeded
}
