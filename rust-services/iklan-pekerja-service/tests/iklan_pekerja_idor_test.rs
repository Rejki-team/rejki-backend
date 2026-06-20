//! Integration tests untuk IDOR protection iklan pekerja.
//!
//! Naming: test_{unit}_given_{cond}_when_{action}_then_{expect}

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
async fn test_delete_given_other_user_when_delete_then_404() {
    let (app, pool) = setup().await;
    let owner = seed_user(&pool, "idor1a").await;
    let attacker = seed_user(&pool, "idor1b").await;

    // Owner creates iklan
    let create_resp = app
        .clone()
        .oneshot(post_authed(
            "/api/v1/pekerja",
            &owner.access_token,
            serde_json::json!({
                "nama": "Owner's Iklan",
                "keahlian": ["Rust"],
                "deskripsi": "Owner's desc",
                "lokasi": "Jakarta"
            }),
        ))
        .await
        .unwrap();
    let id = body_json(create_resp).await["data"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Attacker tries to delete → 404
    let resp = app
        .clone()
        .oneshot(delete_authed(
            &format!("/api/v1/pekerja/{id}"),
            &attacker.access_token,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // Iklan still exists
    let get_resp = app
        .oneshot(get_anon(&format!("/api/v1/pekerja/{id}")))
        .await
        .unwrap();
    assert_eq!(get_resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_patch_given_other_user_when_patch_then_404() {
    let (app, pool) = setup().await;
    let owner = seed_user(&pool, "idor2a").await;
    let attacker = seed_user(&pool, "idor2b").await;

    let create_resp = app
        .clone()
        .oneshot(post_authed(
            "/api/v1/pekerja",
            &owner.access_token,
            serde_json::json!({
                "nama": "Owner's Iklan",
                "keahlian": ["Rust"],
                "deskripsi": "Owner's desc",
                "lokasi": "Jakarta"
            }),
        ))
        .await
        .unwrap();
    let id = body_json(create_resp).await["data"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Attacker tries to patch → 404
    let resp = app
        .clone()
        .oneshot(common::patch_authed(
            &format!("/api/v1/pekerja/{id}"),
            &attacker.access_token,
            serde_json::json!({"nama": "Hacked Name"}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
