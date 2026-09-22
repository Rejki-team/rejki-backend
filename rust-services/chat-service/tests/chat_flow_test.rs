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
    body_json, build_test_app, clean_test_data, get_authed, patch_authed, post_authed, seed_user,
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
    assert!(!data["id"].as_str().unwrap().is_empty());
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
                "content_type": "text",
                "content": "Hello",
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = body_json(resp).await;
    assert!(body["success"].as_bool().unwrap());
    assert_eq!(body["data"]["content_type"], "text");
    assert_eq!(body["data"]["content"], "Hello");
    assert_eq!(
        body["data"]["sender_id"].as_str().unwrap(),
        user_a.id.to_string()
    );
}

// ── Send location & photo (P4.1/P4.2, F-19) ─────────────────────────────

#[tokio::test]
async fn test_send_message_given_location_when_send_then_lat_lng_tersimpan() {
    let (app, pool) = setup().await;
    let user_a = seed_user(&pool, "chat4a").await;
    let user_b = seed_user(&pool, "chat4b").await;

    let resp = app
        .clone()
        .oneshot(post_authed(
            "/api/v1/chat/conversations",
            &user_a.access_token,
            serde_json::json!({ "other_user_id": user_b.id }),
        ))
        .await
        .unwrap();
    let conv_id = body_json(resp).await["data"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let resp = app
        .clone()
        .oneshot(post_authed(
            &format!("/api/v1/chat/conversations/{conv_id}/messages"),
            &user_a.access_token,
            serde_json::json!({
                "content_type": "location",
                "lat": -6.2,
                "lng": 106.8,
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = body_json(resp).await;
    assert_eq!(body["data"]["content_type"], "location");
    assert_eq!(body["data"]["lat"].as_f64().unwrap(), -6.2);
    assert_eq!(body["data"]["lng"].as_f64().unwrap(), 106.8);
    assert!(body["data"]["content"].is_null());
}

#[tokio::test]
async fn test_send_message_given_invalid_coordinates_when_send_then_422() {
    let (app, pool) = setup().await;
    let user_a = seed_user(&pool, "chat5a").await;
    let user_b = seed_user(&pool, "chat5b").await;

    let resp = app
        .clone()
        .oneshot(post_authed(
            "/api/v1/chat/conversations",
            &user_a.access_token,
            serde_json::json!({ "other_user_id": user_b.id }),
        ))
        .await
        .unwrap();
    let conv_id = body_json(resp).await["data"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let resp = app
        .clone()
        .oneshot(post_authed(
            &format!("/api/v1/chat/conversations/{conv_id}/messages"),
            &user_a.access_token,
            serde_json::json!({ "content_type": "location", "lat": 999.0, "lng": 106.8 }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_send_message_given_photo_when_send_then_object_key_tersimpan() {
    let (app, pool) = setup().await;
    let user_a = seed_user(&pool, "chat6a").await;
    let user_b = seed_user(&pool, "chat6b").await;

    let resp = app
        .clone()
        .oneshot(post_authed(
            "/api/v1/chat/conversations",
            &user_a.access_token,
            serde_json::json!({ "other_user_id": user_b.id }),
        ))
        .await
        .unwrap();
    let conv_id = body_json(resp).await["data"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let resp = app
        .clone()
        .oneshot(post_authed(
            &format!("/api/v1/chat/conversations/{conv_id}/messages"),
            &user_a.access_token,
            serde_json::json!({
                "content_type": "photo",
                "photo_object_key": "chat-photo/2026/09/test-object-key.jpg",
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = body_json(resp).await;
    assert_eq!(body["data"]["content_type"], "photo");
    assert_eq!(
        body["data"]["photo_object_key"],
        "chat-photo/2026/09/test-object-key.jpg"
    );
}

// ── Membership (IDOR, P4.0) ──────────────────────────────────────────────

#[tokio::test]
async fn test_send_message_given_non_participant_when_send_then_404() {
    let (app, pool) = setup().await;
    let user_a = seed_user(&pool, "chat7a").await;
    let user_b = seed_user(&pool, "chat7b").await;
    let intruder = seed_user(&pool, "chat7c").await;

    let resp = app
        .clone()
        .oneshot(post_authed(
            "/api/v1/chat/conversations",
            &user_a.access_token,
            serde_json::json!({ "other_user_id": user_b.id }),
        ))
        .await
        .unwrap();
    let conv_id = body_json(resp).await["data"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let resp = app
        .clone()
        .oneshot(post_authed(
            &format!("/api/v1/chat/conversations/{conv_id}/messages"),
            &intruder.access_token,
            serde_json::json!({ "content_type": "text", "content": "aku bukan participant" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ── Akhiri percakapan manual (P4.4) ──────────────────────────────────────

#[tokio::test]
async fn test_end_conversation_given_participant_when_akhiri_then_ended_at_terisi() {
    let (app, pool) = setup().await;
    let user_a = seed_user(&pool, "chat8a").await;
    let user_b = seed_user(&pool, "chat8b").await;

    let resp = app
        .clone()
        .oneshot(post_authed(
            "/api/v1/chat/conversations",
            &user_a.access_token,
            serde_json::json!({ "other_user_id": user_b.id }),
        ))
        .await
        .unwrap();
    let conv_id = body_json(resp).await["data"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let resp = app
        .clone()
        .oneshot(patch_authed(
            &format!("/api/v1/chat/conversations/{conv_id}/akhiri"),
            &user_a.access_token,
            serde_json::json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert!(!body["data"]["ended_at"].is_null());

    // Idempoten — ditekan lagi tidak error, ended_at tetap terisi.
    let resp2 = app
        .clone()
        .oneshot(patch_authed(
            &format!("/api/v1/chat/conversations/{conv_id}/akhiri"),
            &user_a.access_token,
            serde_json::json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(resp2.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_end_conversation_given_non_participant_when_akhiri_then_404() {
    let (app, pool) = setup().await;
    let user_a = seed_user(&pool, "chat9a").await;
    let user_b = seed_user(&pool, "chat9b").await;
    let intruder = seed_user(&pool, "chat9c").await;

    let resp = app
        .clone()
        .oneshot(post_authed(
            "/api/v1/chat/conversations",
            &user_a.access_token,
            serde_json::json!({ "other_user_id": user_b.id }),
        ))
        .await
        .unwrap();
    let conv_id = body_json(resp).await["data"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let resp = app
        .clone()
        .oneshot(patch_authed(
            &format!("/api/v1/chat/conversations/{conv_id}/akhiri"),
            &intruder.access_token,
            serde_json::json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ── Auto-end & retensi (P4.3, P4.5) — handler diuji langsung (waktu di-mock:
// tidak menunggu 2x24 jam/60 hari nyata, cukup panggil handler & verifikasi efek). ──

#[tokio::test]
async fn test_auto_end_handler_given_active_conversation_when_handle_then_ended_at_terisi() {
    use chat_service::application::scheduled_jobs::AutoEndHandler;
    use chat_service::domain::repository::ChatRepository;
    use chat_service::PgChatRepository;
    use common_scheduler::JobHandler;

    let (_app, pool) = setup().await;
    let user_a = seed_user(&pool, "chat10a").await;
    let user_b = seed_user(&pool, "chat10b").await;

    let repo = std::sync::Arc::new(PgChatRepository::new(pool.clone()));
    let (conv, is_new) = repo
        .find_or_create_conversation(user_a.id, user_b.id, None, None)
        .await
        .unwrap();
    assert!(is_new);

    let handler = AutoEndHandler { repo: repo.clone() };
    handler
        .handle(&serde_json::json!({ "conversation_id": conv.id }))
        .await
        .unwrap();

    let after = repo
        .find_conversation_by_id(conv.id)
        .await
        .unwrap()
        .expect("conversation harus masih ada");
    assert!(after.ended_at.is_some());

    // Idempoten — dipanggil lagi tidak error, tidak menimpa ended_at yang sudah ada.
    let ended_at_first = after.ended_at.unwrap();
    handler
        .handle(&serde_json::json!({ "conversation_id": conv.id }))
        .await
        .unwrap();
    let after2 = repo
        .find_conversation_by_id(conv.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(after2.ended_at.unwrap(), ended_at_first);
}

#[tokio::test]
async fn test_retention_purge_handler_given_conversation_with_messages_when_handle_then_cascade_deleted(
) {
    use chat_service::application::scheduled_jobs::RetentionPurgeHandler;
    use chat_service::domain::entity::{MessageContentType, NewMessage};
    use chat_service::domain::repository::ChatRepository;
    use chat_service::PgChatRepository;
    use common_scheduler::JobHandler;

    let (_app, pool) = setup().await;
    let user_a = seed_user(&pool, "chat11a").await;
    let user_b = seed_user(&pool, "chat11b").await;

    let repo = std::sync::Arc::new(PgChatRepository::new(pool.clone()));
    let (conv, _) = repo
        .find_or_create_conversation(user_a.id, user_b.id, None, None)
        .await
        .unwrap();

    // P4.9 (Audit Finding #6): kirim pesan SEBELUM purge — sebelumnya test ini menguji
    // conversation kosong, tidak pernah membuktikan FK `ON DELETE CASCADE` benar-benar
    // menghapus `chat.messages`, hanya DDL-nya yang ada.
    repo.save_message(
        conv.id,
        user_a.id,
        &NewMessage {
            content_type: MessageContentType::Text,
            content: Some("Pesan sebelum retensi".into()),
            lat: None,
            lng: None,
            photo_object_key: None,
        },
    )
    .await
    .unwrap();

    let handler = RetentionPurgeHandler { repo: repo.clone() };
    handler
        .handle(&serde_json::json!({ "conversation_id": conv.id }))
        .await
        .unwrap();

    let after = repo.find_conversation_by_id(conv.id).await.unwrap();
    assert!(
        after.is_none(),
        "conversation harus sudah terhapus (hard delete)"
    );

    let remaining_messages = repo.list_messages(conv.id, 10, None, None).await.unwrap();
    assert!(
        remaining_messages.is_empty(),
        "messages harus ikut terhapus via FK CASCADE, ditemukan: {remaining_messages:?}"
    );

    // Idempoten — dipanggil lagi (mis. re-delivery) tidak error meski sudah tak ada.
    handler
        .handle(&serde_json::json!({ "conversation_id": conv.id }))
        .await
        .unwrap();
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
                "content_type": "text",
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

// ── List conversations (P4.10, F-18) ─────────────────────────────────────

#[tokio::test]
async fn test_list_conversations_given_message_when_list_then_has_unread_true() {
    let (app, pool) = setup().await;
    let user_a = seed_user(&pool, "chat4a").await;
    let user_b = seed_user(&pool, "chat4b").await;

    let resp = app
        .clone()
        .oneshot(post_authed(
            "/api/v1/chat/conversations",
            &user_a.access_token,
            serde_json::json!({ "other_user_id": user_b.id }),
        ))
        .await
        .unwrap();
    let conv_id = body_json(resp).await["data"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let resp = app
        .clone()
        .oneshot(post_authed(
            &format!("/api/v1/chat/conversations/{conv_id}/messages"),
            &user_a.access_token,
            serde_json::json!({ "content_type": "text", "content": "Halo" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // user_b belum baca -> has_unread true
    let resp = app
        .clone()
        .oneshot(get_authed(
            "/api/v1/chat/conversations",
            &user_b.access_token,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let items = body["data"].as_array().unwrap();
    let item = items
        .iter()
        .find(|i| i["id"].as_str().unwrap() == conv_id)
        .expect("conversation harus ada di daftar user_b");
    assert!(item["has_unread"].as_bool().unwrap());
    assert_eq!(item["last_message"]["content"].as_str().unwrap(), "Halo");

    // user_a sendiri yang kirim -> tidak dianggap unread buat dirinya
    let resp = app
        .clone()
        .oneshot(get_authed(
            "/api/v1/chat/conversations",
            &user_a.access_token,
        ))
        .await
        .unwrap();
    let body = body_json(resp).await;
    let item = body["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["id"].as_str().unwrap() == conv_id)
        .unwrap();
    assert!(!item["has_unread"].as_bool().unwrap());
}

#[tokio::test]
async fn test_list_conversations_given_unrelated_user_when_list_then_not_included() {
    let (app, pool) = setup().await;
    let user_a = seed_user(&pool, "chat5a").await;
    let user_b = seed_user(&pool, "chat5b").await;
    let outsider = seed_user(&pool, "chat5c").await;

    let resp = app
        .clone()
        .oneshot(post_authed(
            "/api/v1/chat/conversations",
            &user_a.access_token,
            serde_json::json!({ "other_user_id": user_b.id }),
        ))
        .await
        .unwrap();
    let conv_id = body_json(resp).await["data"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let resp = app
        .clone()
        .oneshot(get_authed(
            "/api/v1/chat/conversations",
            &outsider.access_token,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let items = body["data"].as_array().unwrap();
    assert!(
        !items.iter().any(|i| i["id"].as_str().unwrap() == conv_id),
        "percakapan orang lain tidak boleh muncul di daftar outsider"
    );
}

// ── Mark read (P4.11, F-18) ───────────────────────────────────────────────

#[tokio::test]
async fn test_mark_read_given_conversation_when_mark_then_unread_becomes_false() {
    let (app, pool) = setup().await;
    let user_a = seed_user(&pool, "chat6a").await;
    let user_b = seed_user(&pool, "chat6b").await;

    let resp = app
        .clone()
        .oneshot(post_authed(
            "/api/v1/chat/conversations",
            &user_a.access_token,
            serde_json::json!({ "other_user_id": user_b.id }),
        ))
        .await
        .unwrap();
    let conv_id = body_json(resp).await["data"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    app.clone()
        .oneshot(post_authed(
            &format!("/api/v1/chat/conversations/{conv_id}/messages"),
            &user_a.access_token,
            serde_json::json!({ "content_type": "text", "content": "Halo" }),
        ))
        .await
        .unwrap();

    let resp = app
        .clone()
        .oneshot(patch_authed(
            &format!("/api/v1/chat/conversations/{conv_id}/read"),
            &user_b.access_token,
            serde_json::json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = app
        .clone()
        .oneshot(get_authed(
            "/api/v1/chat/conversations",
            &user_b.access_token,
        ))
        .await
        .unwrap();
    let body = body_json(resp).await;
    let item = body["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["id"].as_str().unwrap() == conv_id)
        .unwrap();
    assert!(!item["has_unread"].as_bool().unwrap());
}

#[tokio::test]
async fn test_mark_read_given_other_user_conversation_when_mark_then_404() {
    let (app, pool) = setup().await;
    let user_a = seed_user(&pool, "chat7a").await;
    let user_b = seed_user(&pool, "chat7b").await;
    let outsider = seed_user(&pool, "chat7c").await;

    let resp = app
        .clone()
        .oneshot(post_authed(
            "/api/v1/chat/conversations",
            &user_a.access_token,
            serde_json::json!({ "other_user_id": user_b.id }),
        ))
        .await
        .unwrap();
    let conv_id = body_json(resp).await["data"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let resp = app
        .clone()
        .oneshot(patch_authed(
            &format!("/api/v1/chat/conversations/{conv_id}/read"),
            &outsider.access_token,
            serde_json::json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
