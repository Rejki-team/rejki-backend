//! Integration tests untuk chat-service — conversation & message flow.
//!
//! Jalankan:
//!   cargo test --test chat_flow_test -- --test-threads=1
//!
//! Prerequisite:
//!   - PostgreSQL test DB hidup (DATABASE_URL dari .env.test)
//!   - Migrasi sudah dijalankan
//!   - RSA key pair ada di ./keys/

mod common;

use axum::http::StatusCode;
use common::{
    body_json, build_test_app, clean_test_data, get_authed, post_authed, seed_user, test_pool,
};
use tower::ServiceExt;

async fn setup() -> (axum::Router, sqlx::PgPool) {
    let _ = dotenvy::from_filename(".env.test");
    let pool = test_pool().await;
    clean_test_data(&pool).await;
    let app = build_test_app(pool.clone()).await;
    (app, pool)
}

// ── Get or create conversation ───────────────────────────────────────────

#[tokio::test]
async fn test_get_or_create_given_valid_users_when_create_then_200() {
    let (app, pool) = setup().await;
    let user_a = seed_user(&pool, "chat1a").await;
    let user_b = seed_user(&pool, "chat1b").await;

    let resp = app
        .clone()
        .oneshot(post_authed(
            "/api/v1/chat/conversations",
            &user_a.access_token,
            serde_json::json!({
                "other_user_id": user_b.id,
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_json(resp).await;
    assert!(body["success"].as_bool().unwrap());
    let data = &body["data"];
    assert!(data["id"].as_str().unwrap().len() > 0);
    let user_a_str = user_a.id.to_string();
    let user_b_str = user_b.id.to_string();
    let returned_a = data["user_a"].as_str().unwrap();
    let returned_b = data["user_b"].as_str().unwrap();
    // user_a < user_b is enforced by the repo, so either order is fine
    let ids = [user_a_str.as_str(), user_b_str.as_str()];
    assert!(
        ids.contains(&returned_a),
        "{returned_a} should be one of user_a/user_b"
    );
    assert!(
        ids.contains(&returned_b),
        "{returned_b} should be one of user_a/user_b"
    );
}

// ── Send message ─────────────────────────────────────────────────────────

#[tokio::test]
async fn test_send_message_given_existing_conversation_when_send_then_201() {
    let (app, pool) = setup().await;
    let user_a = seed_user(&pool, "chat2a").await;
    let user_b = seed_user(&pool, "chat2b").await;

    // Create conversation first
    let resp = app
        .clone()
        .oneshot(post_authed(
            "/api/v1/chat/conversations",
            &user_a.access_token,
            serde_json::json!({
                "other_user_id": user_b.id,
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let conv_id = body_json(resp).await["data"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Send message
    let resp = app
        .clone()
        .oneshot(post_authed(
            &format!("/api/v1/chat/conversations/{conv_id}/messages"),
            &user_a.access_token,
            serde_json::json!({
                "content": "Hello",
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = body_json(resp).await;
    assert!(body["success"].as_bool().unwrap());
    assert_eq!(body["data"]["content"], "Hello");
    assert_eq!(
        body["data"]["sender_id"].as_str().unwrap(),
        user_a.id.to_string()
    );
}

// ── List messages ────────────────────────────────────────────────────────

#[tokio::test]
async fn test_list_messages_given_conversation_with_messages_when_list_then_200() {
    let (app, pool) = setup().await;
    let user_a = seed_user(&pool, "chat3a").await;
    let user_b = seed_user(&pool, "chat3b").await;

    // Create conversation
    let resp = app
        .clone()
        .oneshot(post_authed(
            "/api/v1/chat/conversations",
            &user_a.access_token,
            serde_json::json!({
                "other_user_id": user_b.id,
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let conv_id = body_json(resp).await["data"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Send a message
    let resp = app
        .clone()
        .oneshot(post_authed(
            &format!("/api/v1/chat/conversations/{conv_id}/messages"),
            &user_a.access_token,
            serde_json::json!({
                "content": "Hello from test",
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // List messages
    let resp = app
        .clone()
        .oneshot(get_authed(
            &format!("/api/v1/chat/conversations/{conv_id}/messages"),
            &user_a.access_token,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_json(resp).await;
    assert!(body["success"].as_bool().unwrap());
    let messages = body["data"]["messages"].as_array().unwrap();
    assert!(!messages.is_empty(), "seharusnya ada minimal 1 pesan");
    assert!(messages[0]["content"].as_str().unwrap().contains("Hello"));
}
