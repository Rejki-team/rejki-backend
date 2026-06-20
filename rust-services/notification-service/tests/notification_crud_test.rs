//! Integration tests untuk notification-service — CRUD endpoint, send, device token.
//!
//! Jalankan:
//!   cargo test --test notification_crud_test -- --test-threads=1
//!
//! Prerequisite:
//!   - PostgreSQL test DB hidup (DATABASE_URL dari .env.test)
//!   - Migrasi sudah dijalankan
//!   - RSA key pair ada di ./keys/

mod common;

use axum::{body::Body, http::Request, http::StatusCode};
use common::{
    body_json, build_test_app, clean_test_data, delete_authed, get_anon, get_authed,
    patch_authed_empty, post_authed, seed_notification, seed_user, test_pool,
};
use tower::ServiceExt;
use uuid::Uuid;

async fn setup() -> (axum::Router, sqlx::PgPool) {
    let _ = dotenvy::from_filename(".env.test");
    let pool = test_pool().await;
    clean_test_data(&pool).await;
    let app = build_test_app(pool.clone()).await;
    (app, pool)
}

// ── List notifications dengan auth ───────────────────────────────────────

#[tokio::test]
async fn test_list_given_authed_user_when_list_then_200() {
    let (app, pool) = setup().await;
    let user = seed_user(&pool, "lst1").await;

    // Seed a notification for this user
    seed_notification(&pool, user.id, "lst1").await;

    let resp = app
        .clone()
        .oneshot(get_authed("/api/v1/notif", &user.access_token))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_json(resp).await;
    assert!(body["success"].as_bool().unwrap());
    let items = body["data"].as_array().expect("data harus berupa array");
    assert!(!items.is_empty(), "seharusnya ada minimal 1 notifikasi");

    // Verify structure
    let notif = &items[0];
    assert!(notif["id"].as_str().unwrap().len() > 0);
    assert!(notif["title"].as_str().unwrap().contains("lst1"));
    assert!(!notif["is_read"].as_bool().unwrap());
}

// ── List notifications tanpa auth → 401 ──────────────────────────────────

#[tokio::test]
async fn test_list_given_no_auth_when_list_then_401() {
    let (app, _pool) = setup().await;

    let resp = app
        .clone()
        .oneshot(get_anon("/api/v1/notif"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ── Mark read existing notification ──────────────────────────────────────

#[tokio::test]
async fn test_mark_read_given_existing_notification_when_mark_then_200() {
    let (app, pool) = setup().await;
    let user = seed_user(&pool, "mrk2").await;
    let notif_id = seed_notification(&pool, user.id, "mrk2").await;

    let resp = app
        .clone()
        .oneshot(patch_authed_empty(
            &format!("/api/v1/notif/{notif_id}/read"),
            &user.access_token,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Verify: notification should now be read
    let resp = app
        .clone()
        .oneshot(get_authed("/api/v1/notif", &user.access_token))
        .await
        .unwrap();
    let body = body_json(resp).await;
    let items = body["data"].as_array().unwrap();
    let updated = items
        .iter()
        .find(|n| n["id"].as_str().unwrap() == notif_id.to_string());
    assert!(updated.is_some(), "notifikasi harus ada di list");
    assert!(
        updated.unwrap()["is_read"].as_bool().unwrap(),
        "notifikasi harus sudah terbaca"
    );
}

// ── Mark read nonexistent notification → 200 (silent no-op) ──────────────

#[tokio::test]
async fn test_mark_read_given_nonexistent_id_when_mark_then_200_noop() {
    let (app, pool) = setup().await;
    let user = seed_user(&pool, "mrk1").await;
    let fake_id = uuid::Uuid::now_v7();

    let resp = app
        .clone()
        .oneshot(patch_authed_empty(
            &format!("/api/v1/notif/{fake_id}/read"),
            &user.access_token,
        ))
        .await
        .unwrap();
    // mark_read mengupdate is_read=true via UPDATE; sqlx execute sukses
    // walau 0 rows affected. Handler tidak mengembalikan error → 200 OK.
    assert_eq!(resp.status(), StatusCode::OK);
}

// ── Register device token ────────────────────────────────────────────────

#[tokio::test]
async fn test_register_device_token_given_valid_input_when_register_then_201() {
    let (app, pool) = setup().await;
    let user = seed_user(&pool, "dev1").await;

    let resp = app
        .clone()
        .oneshot(post_authed(
            "/api/v1/notif/tokens",
            &user.access_token,
            serde_json::json!({
                "token": "fcm-token-abc123",
                "platform": "android",
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = body_json(resp).await;
    assert!(body["success"].as_bool().unwrap());
    let data = &body["data"];
    assert_eq!(data["token"], "fcm-token-abc123");
    assert_eq!(data["platform"], "android");
}

// ── Register device token tanpa auth → 401 ────────────────────────────────

#[tokio::test]
async fn test_register_device_token_given_no_auth_when_register_then_401() {
    let (app, _pool) = setup().await;

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/notif/tokens")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "token": "fcm-token-abc",
                        "platform": "android",
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ── List device tokens ───────────────────────────────────────────────────

#[tokio::test]
async fn test_list_device_tokens_given_authed_user_when_list_then_200() {
    let (app, pool) = setup().await;
    let user = seed_user(&pool, "dev2").await;

    // List tokens — should return 200 even if empty
    let resp = app
        .clone()
        .oneshot(get_authed("/api/v1/notif/tokens", &user.access_token))
        .await
        .unwrap();
    // 200 OK regardless of whether tokens exist
    assert!(resp.status().is_success());
}

// ── Delete device token ─────────────────────────────────────────────────

#[tokio::test]
async fn test_delete_device_token_given_existing_token_when_delete_then_204() {
    let (app, pool) = setup().await;
    let user = seed_user(&pool, "dev3").await;

    // Register a token first
    let resp = app
        .clone()
        .oneshot(post_authed(
            "/api/v1/notif/tokens",
            &user.access_token,
            serde_json::json!({
                "token": "fcm-token-delete-me",
                "platform": "web",
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Delete token
    let resp = app
        .clone()
        .oneshot(delete_authed(
            "/api/v1/notif/tokens/fcm-token-delete-me",
            &user.access_token,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Verify: token list should be empty
    let resp = app
        .clone()
        .oneshot(get_authed("/api/v1/notif/tokens", &user.access_token))
        .await
        .unwrap();
    let body = body_json(resp).await;
    let tokens = body["data"].as_array().unwrap();
    assert!(
        tokens.is_empty(),
        "seharusnya tidak ada token setelah dihapus"
    );
}

// ── Send notification (admin/internal endpoint) ──────────────────────────

#[tokio::test]
async fn test_send_given_valid_input_when_send_then_202() {
    let (app, pool) = setup().await;
    let user = seed_user(&pool, "snd1").await;

    let resp = app
        .clone()
        .oneshot(post_authed(
            "/api/v1/notif/send",
            &user.access_token,
            serde_json::json!({
                "recipient_id": user.id,
                "title": "Test Notification",
                "body": "This is a test notification body",
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
}

// ── Send notification tanpa auth → 401 ──────────────────────────────────

#[tokio::test]
async fn test_send_given_no_auth_when_send_then_401() {
    let (app, _pool) = setup().await;

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/notif/send")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "recipient_id": Uuid::now_v7(),
                        "title": "Test",
                        "body": "Test body",
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
