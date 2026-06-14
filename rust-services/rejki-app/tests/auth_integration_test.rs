/// Integration tests untuk authorization layer.
///
/// Naming convention: test_{unit}_given_{cond}_when_{action}_then_{expect}
///
/// Jalankan:
///   cargo test --test auth_integration_test -- --test-threads=1
///
/// Prerequisite:
///   - PostgreSQL test DB hidup (DATABASE_URL dari .env.test)
///   - Migrasi sudah dijalankan: sqlx migrate run
///   - RSA key pair ada di ./keys/
mod common;

use axum::http::StatusCode;
use tower::ServiceExt; // oneshot()

// ── Helpers ───────────────────────────────────────────────────────────────────

async fn setup() -> (axum::Router, sqlx::PgPool) {
    let _ = dotenvy::from_filename(".env.test");
    let pool = common::test_pool().await;
    common::fixtures::clean_test_data(&pool).await;
    let app = common::build_test_app(pool.clone()).await;
    (app, pool)
}

// ── Health check ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_health_given_no_auth_when_get_health_then_200() {
    let (app, _) = setup().await;
    let resp = app.oneshot(common::get_anon("/health")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = common::body_json(resp).await;
    assert_eq!(body["status"], "ok");
}

// ── Protected routes: 401 tanpa token ────────────────────────────────────────

#[tokio::test]
async fn test_user_profile_given_no_token_when_get_me_then_401() {
    let (app, _) = setup().await;
    let resp = app
        .oneshot(common::get_anon("/api/v1/users/me"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_chat_given_no_token_when_get_rooms_then_401() {
    let (app, _) = setup().await;
    // Route REST chat yang nyata & protected (require_auth).
    let resp = app
        .oneshot(common::get_anon(
            "/api/v1/chat/conversations/00000000-0000-0000-0000-000000000000/messages",
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_notif_given_no_token_when_get_list_then_401() {
    let (app, _) = setup().await;
    let resp = app
        .oneshot(common::get_anon("/api/v1/notif"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ── Protected routes: 401 dengan token rusak ─────────────────────────────────

#[tokio::test]
async fn test_user_profile_given_bad_token_when_get_me_then_401() {
    let (app, _) = setup().await;
    let resp = app
        .oneshot(common::get_authed("/api/v1/users/me", "not.a.valid.jwt"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ── Protected routes: 200 dengan valid token ──────────────────────────────────

#[tokio::test]
async fn test_user_profile_given_valid_token_when_get_me_then_200() {
    let (app, pool) = setup().await;
    let user = common::seed_user(&pool, "profile-ok").await;

    let resp = app
        .oneshot(common::get_authed("/api/v1/users/me", &user.access_token))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    let body = common::body_json(resp).await;
    assert_eq!(body["success"], true);
    // Profil user_svc menyimpan username (email ada di domain auth, bukan profil).
    assert_eq!(body["data"]["username"], format!("user_{}", "profile-ok"));
}

// ── IDOR protection: delete resource milik user lain → 404 ──────────────────

#[tokio::test]
async fn test_iklan_given_different_owner_when_delete_pekerjaan_then_404() {
    use axum::{body::Body, http::Request};

    let (app, pool) = setup().await;

    let owner = common::seed_user(&pool, "idor-owner").await;
    let attacker = common::seed_user(&pool, "idor-attacker").await;

    // Seed iklan milik owner langsung ke DB
    let iklan_id = uuid::Uuid::now_v7();
    sqlx::query!(
        r#"
        INSERT INTO iklan_pekerjaan.iklan
            (id, poster_id, judul, perusahaan, deskripsi, lokasi, gaji_min, gaji_max, created_at, updated_at)
        VALUES ($1, $2, 'Test Iklan', 'PT Test', 'Deskripsi test', 'Jakarta', 5000000, 10000000, now(), now())
        "#,
        iklan_id,
        owner.id,
    )
    .execute(&pool)
    .await
    .expect("seed iklan failed");

    // Attacker coba DELETE iklan milik owner
    let req = Request::builder()
        .method("DELETE")
        .uri(format!("/api/v1/pekerjaan/{iklan_id}"))
        .header("authorization", format!("Bearer {}", attacker.access_token))
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();

    // 404 — ownership check gagal; tidak bocorkan bahwa resource ada
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ── Owner dapat delete iklan milik sendiri → 204 ──────────────────────────────

#[tokio::test]
async fn test_iklan_given_correct_owner_when_delete_pekerjaan_then_204() {
    use axum::{body::Body, http::Request};

    let (app, pool) = setup().await;
    let owner = common::seed_user(&pool, "idor-self-delete").await;

    let iklan_id = uuid::Uuid::now_v7();
    sqlx::query!(
        r#"
        INSERT INTO iklan_pekerjaan.iklan
            (id, poster_id, judul, perusahaan, deskripsi, lokasi, gaji_min, gaji_max, created_at, updated_at)
        VALUES ($1, $2, 'My Iklan', 'PT Self', 'Deskripsi', 'Bandung', 3000000, 6000000, now(), now())
        "#,
        iklan_id,
        owner.id,
    )
    .execute(&pool)
    .await
    .expect("seed iklan failed");

    let req = Request::builder()
        .method("DELETE")
        .uri(format!("/api/v1/pekerjaan/{iklan_id}"))
        .header("authorization", format!("Bearer {}", owner.access_token))
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

// ── StorageClient wiring (avatar / dokumen KYC) ──────────────────────────────
//
// Membuktikan handler benar-benar mendelegasikan ke StorageClient (bukan lagi
// presigned-URL placeholder). Pada lingkungan tes, MinIO tidak di-set sehingga:
//   - MIME tidak didukung → 400 (validasi domain storage berjalan)
//   - MIME valid → 500 Unavailable (storage nyata dipanggil, bukan URL palsu)

#[tokio::test]
async fn test_avatar_given_invalid_mime_when_request_upload_then_422() {
    use axum::{body::Body, http::Request};
    use serde_json::json;

    let (app, pool) = setup().await;
    let user = common::seed_user(&pool, "avatar-bad-mime").await;

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/users/me/avatar")
        .header("authorization", format!("Bearer {}", user.access_token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({ "mime": "application/pdf", "size_bytes": 1024 }).to_string(),
        ))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    // Validasi MIME domain storage menolak pdf untuk avatar (AppError::Validation → 422).
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_avatar_given_valid_mime_when_storage_unset_then_500() {
    use axum::{body::Body, http::Request};
    use serde_json::json;

    let (app, pool) = setup().await;
    let user = common::seed_user(&pool, "avatar-no-minio").await;

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/users/me/avatar")
        .header("authorization", format!("Bearer {}", user.access_token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({ "mime": "image/jpeg", "size_bytes": 1024 }).to_string(),
        ))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    // StorageClient dipanggil (MinIO tak di-set → Unavailable). Membuktikan
    // wiring nyata: dulu handler mengembalikan 200 dengan URL placeholder.
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn test_document_given_invalid_kind_when_request_upload_then_422() {
    use axum::{body::Body, http::Request};
    use serde_json::json;

    let (app, pool) = setup().await;
    let user = common::seed_user(&pool, "doc-bad-kind").await;

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/users/me/documents")
        .header("authorization", format!("Bearer {}", user.access_token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({ "kind": "paspor", "mime": "image/jpeg", "size_bytes": 1024 }).to_string(),
        ))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    // Hanya 'ktp' / 'selfie' yang valid (AppError::Validation → 422).
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

// ── Request ID header dipropagasi ─────────────────────────────────────────────

#[tokio::test]
async fn test_request_id_given_any_request_when_get_health_then_x_request_id_present() {
    let (app, _) = setup().await;
    let resp = app.oneshot(common::get_anon("/health")).await.unwrap();

    let has_request_id = resp.headers().contains_key("x-request-id");
    assert!(
        has_request_id,
        "x-request-id header harus ada di setiap response"
    );
}

#[tokio::test]
async fn test_request_id_given_client_provided_id_when_get_health_then_echoed_back() {
    use axum::{body::Body, http::Request};

    let (app, _) = setup().await;
    let client_id = "test-request-id-abc123";

    let req = Request::builder()
        .method("GET")
        .uri("/health")
        .header("x-request-id", client_id)
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    let echoed = resp
        .headers()
        .get("x-request-id")
        .unwrap()
        .to_str()
        .unwrap();
    assert_eq!(echoed, client_id);
}
