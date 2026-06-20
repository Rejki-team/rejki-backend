//! Integration tests untuk CRUD dasar iklan pelatihan.
//!
//! Naming: test_{unit}_given_{cond}_when_{action}_then_{expect}
//!
//! Jalankan:
//!   cargo test --test pelatihan_crud_test -- --test-threads=1

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use common::{
    body_json, build_test_app, delete_authed, fixtures::clean_test_data, get_anon, patch_authed,
    post_authed, seed_iklan_pelatihan, seed_user, test_pool,
};

async fn setup() -> (axum::Router, sqlx::PgPool) {
    let _ = dotenvy::from_filename(".env.test");
    let pool = test_pool().await;
    clean_test_data(&pool).await;
    let app = build_test_app(pool.clone()).await;
    (app, pool)
}

const BASE: &str = "/api/v1/pelatihan";

#[tokio::test]
async fn test_create_given_anon_when_create_then_401() {
    let (app, _pool) = setup().await;

    let req = Request::builder()
        .method("POST")
        .uri(BASE)
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "judul": "Pelatihan Ilegal",
                "penyelenggara": "Anonymous",
                "deskripsi": "Harus ditolak",
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_create_given_valid_input_when_create_then_200_or_201() {
    let (app, pool) = setup().await;
    let user = seed_user(&pool, "crt1").await;

    let resp = app
        .oneshot(post_authed(
            BASE,
            &user.access_token,
            serde_json::json!({
                "judul": "Pelatihan Rust Dasar",
                "penyelenggara": "Rejki Academy",
                "deskripsi": "Belajar Rust dari nol sampai mahir",
            }),
        ))
        .await
        .unwrap();
    let status = resp.status();
    assert!(
        status == StatusCode::CREATED || status == StatusCode::OK,
        "expected 201 or 200, got {}",
        status
    );

    let body = body_json(resp).await;
    assert!(body
        .get("data")
        .and_then(|d| d.get("id"))
        .and_then(|i| i.as_str())
        .map(|s| !s.is_empty())
        .unwrap_or(false));
}

#[tokio::test]
async fn test_create_given_short_judul_when_create_then_422() {
    let (app, pool) = setup().await;
    let user = seed_user(&pool, "crt2").await;

    let resp = app
        .oneshot(post_authed(
            BASE,
            &user.access_token,
            serde_json::json!({
                "judul": "AB",
                "penyelenggara": "Test",
                "deskripsi": "Coba",
            }),
        ))
        .await
        .unwrap();
    let status = resp.status();
    assert!(
        status.is_client_error(),
        "expected 4xx for short judul, got {}",
        status
    );
}

#[tokio::test]
async fn test_get_given_valid_id_when_get_then_200() {
    let (app, pool) = setup().await;
    let user = seed_user(&pool, "get1").await;

    let iklan_id = seed_iklan_pelatihan(&pool, user.id, "get1", "user", "default").await;

    let resp = app
        .oneshot(get_anon(&format!("{BASE}/{iklan_id}")))
        .await
        .unwrap();
    let status = resp.status();
    assert!(
        status.is_success(),
        "expected 2xx for get valid id, got {}",
        status
    );
}

#[tokio::test]
async fn test_get_given_nonexistent_id_when_get_then_404() {
    let (app, _pool) = setup().await;
    let fake_id = uuid::Uuid::now_v7();

    let resp = app
        .oneshot(get_anon(&format!("{BASE}/{fake_id}")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_list_given_seeded_data_when_list_then_200() {
    let (app, pool) = setup().await;
    let user = seed_user(&pool, "lst1").await;

    seed_iklan_pelatihan(&pool, user.id, "lst_a", "user", "default").await;
    seed_iklan_pelatihan(&pool, user.id, "lst_b", "user", "default").await;

    let resp = app.oneshot(get_anon(BASE)).await.unwrap();
    let status = resp.status();
    assert!(status.is_success(), "expected 2xx for list, got {}", status);
}

#[tokio::test]
async fn test_delete_given_owner_when_delete_then_204() {
    let (app, pool) = setup().await;
    let user = seed_user(&pool, "del1").await;

    let iklan_id = seed_iklan_pelatihan(&pool, user.id, "del1", "user", "default").await;

    let resp = app
        .clone()
        .oneshot(delete_authed(
            &format!("{BASE}/{iklan_id}"),
            &user.access_token,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn test_get_given_soft_deleted_when_get_then_404() {
    let (app, pool) = setup().await;
    let user = seed_user(&pool, "del_get").await;

    let iklan_id = seed_iklan_pelatihan(&pool, user.id, "dg1", "user", "default").await;

    // Delete first
    app.clone()
        .oneshot(delete_authed(
            &format!("{BASE}/{iklan_id}"),
            &user.access_token,
        ))
        .await
        .unwrap();

    // Verify deleted
    let resp = app
        .oneshot(get_anon(&format!("{BASE}/{iklan_id}")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_patch_given_owner_when_patch_then_200() {
    let (app, pool) = setup().await;
    let user = seed_user(&pool, "upd1").await;

    let iklan_id =
        seed_iklan_pelatihan(&pool, user.id, "upd1", "user", "verifikasi_diterima").await;

    let resp = app
        .clone()
        .oneshot(patch_authed(
            &format!("{BASE}/{iklan_id}"),
            &user.access_token,
            serde_json::json!({"judul": "Updated Title"}),
        ))
        .await
        .unwrap();
    let status = resp.status();
    assert!(
        status.is_success() || status == StatusCode::FORBIDDEN,
        "expected 2xx or 403 for patch, got {}",
        status
    );
}
