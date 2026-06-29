//! Integration tests untuk CRUD iklan pekerja.
//!
//! Naming: test_{unit}_given_{cond}_when_{action}_then_{expect}
//!
//! Jalankan:
//!   cargo test --test iklan_pekerja_crud_test -- --test-threads=1

mod common;

use axum::http::StatusCode;
use tower::ServiceExt;

use common::{
    body_json, build_test_app, delete_authed, fixtures::clean_test_data, fixtures::seed_user,
    get_anon, post_authed, test_pool,
};

async fn setup() -> (axum::Router, sqlx::PgPool) {
    let _ = dotenvy::from_filename(".env.test");
    let pool = test_pool().await;
    clean_test_data(&pool).await;
    let app = build_test_app(pool.clone()).await;
    (app, pool)
}

#[tokio::test]
async fn test_create_given_valid_input_when_create_then_201() {
    let (app, pool) = setup().await;
    let user = seed_user(&pool, "crt1").await;

    let resp = app
        .oneshot(post_authed(
            "/api/v1/pekerja",
            &user.access_token,
            serde_json::json!({
                "nama": "Budi Santoso",
                "keahlian": ["Rust", "Axum"],
                "deskripsi": "Backend engineer",
                "lokasi": "Jakarta"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let data = body_json(resp).await;
    assert!(!data["data"]["id"].as_str().unwrap().is_empty());
    assert_eq!(data["data"]["nama"], "Budi Santoso");
}

#[tokio::test]
async fn test_get_given_valid_id_when_get_then_200() {
    let (app, pool) = setup().await;
    let user = seed_user(&pool, "get1").await;

    // Create first
    let create_resp = app
        .clone()
        .oneshot(post_authed(
            "/api/v1/pekerja",
            &user.access_token,
            serde_json::json!({
                "nama": "Test Worker",
                "keahlian": ["Rust"],
                "deskripsi": "Test",
                "lokasi": "Jakarta"
            }),
        ))
        .await
        .unwrap();
    let id = body_json(create_resp).await["data"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let resp = app
        .oneshot(get_anon(&format!("/api/v1/pekerja/{id}")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let data = body_json(resp).await;
    assert_eq!(data["data"]["id"], id);
}

#[tokio::test]
async fn test_list_given_created_iklan_when_list_then_200() {
    let (app, pool) = setup().await;
    let user = seed_user(&pool, "lst1").await;

    app.clone()
        .oneshot(post_authed(
            "/api/v1/pekerja",
            &user.access_token,
            serde_json::json!({
                "nama": "Worker Satu",
                "keahlian": ["Rust"],
                "deskripsi": "Test",
                "lokasi": "Jakarta"
            }),
        ))
        .await
        .unwrap();

    let resp = app.oneshot(get_anon("/api/v1/pekerja")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let data = body_json(resp).await;
    let items = data["data"].as_array().unwrap();
    assert!(!items.is_empty());
}

#[tokio::test]
async fn test_delete_given_owner_when_delete_then_204() {
    let (app, pool) = setup().await;
    let user = seed_user(&pool, "del1").await;

    let create_resp = app
        .clone()
        .oneshot(post_authed(
            "/api/v1/pekerja",
            &user.access_token,
            serde_json::json!({
                "nama": "To Delete",
                "keahlian": ["Rust"],
                "deskripsi": "Test",
                "lokasi": "Jakarta"
            }),
        ))
        .await
        .unwrap();
    let id = body_json(create_resp).await["data"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let resp = app
        .clone()
        .oneshot(delete_authed(
            &format!("/api/v1/pekerja/{id}"),
            &user.access_token,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Verify deleted
    let get_resp = app
        .oneshot(get_anon(&format!("/api/v1/pekerja/{id}")))
        .await
        .unwrap();
    assert_eq!(get_resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_given_non_owner_when_delete_then_404() {
    let (app, pool) = setup().await;
    let owner = seed_user(&pool, "del2a").await;
    let other = seed_user(&pool, "del2b").await;

    let create_resp = app
        .clone()
        .oneshot(post_authed(
            "/api/v1/pekerja",
            &owner.access_token,
            serde_json::json!({
                "nama": "Owner's Iklan",
                "keahlian": ["Rust"],
                "deskripsi": "Test",
                "lokasi": "Jakarta"
            }),
        ))
        .await
        .unwrap();
    let id = body_json(create_resp).await["data"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let resp = app
        .oneshot(delete_authed(
            &format!("/api/v1/pekerja/{id}"),
            &other.access_token,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
