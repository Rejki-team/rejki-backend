//! Integration tests untuk bulk suspend pengguna + auto-purge dokumen KYC
//! (extend-user-suspension-bulk-purge).
//!
//! Naming: test_{unit}_given_{cond}_when_{action}_then_{expect}
//!
//! Jalankan:
//!   cargo test --test bulk_suspend_test -- --test-threads=1
//!
//! Prerequisite:
//!   - PostgreSQL test DB hidup (DATABASE_URL dari .env.test)
//!   - Migrasi semua service sudah dijalankan
//!   - RSA key pair ada di ./keys/

#![allow(dead_code)]

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

use common::{
    body_json, build_test_app, fixtures::clean_test_data, seed_admin, seed_kyc_submission,
    seed_user, test_pool,
};

async fn setup() -> (axum::Router, sqlx::PgPool) {
    let _ = dotenvy::from_filename(".env.test");
    let pool = test_pool().await;
    clean_test_data(&pool).await;
    let app = build_test_app(pool.clone()).await;
    (app, pool)
}

fn post_json_authed(uri: &str, token: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn post_json_anon(uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

// ── Bulk suspend: happy path ─────────────────────────────────────────────────

#[tokio::test]
async fn test_bulk_suspend_given_valid_users_when_suspend_then_all_success() {
    let (app, pool) = setup().await;
    let admin = seed_admin(&pool, "bs_admin").await;
    let u1 = seed_user(&pool, "bs_user1").await;
    let u2 = seed_user(&pool, "bs_user2").await;

    let resp = app
        .oneshot(post_json_authed(
            "/api/v1/auth/admin/users/suspend",
            &admin.access_token,
            serde_json::json!({
                "user_ids": [u1.id, u2.id],
                "permanent": true,
                "reason": "Melanggar ketentuan platform bersama-sama",
                "evidence_object_key": "uploads/suspension-evidence/evidence1.pdf"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_json(resp).await;
    let results = body["data"]["results"].as_array().unwrap();
    assert_eq!(results.len(), 2);

    for r in results {
        assert_eq!(r["success"], true, "tiap user harus sukses: {r:?}");
    }

    // Verify DB: users suspended (status = 'suspended_permanent')
    for uid in &[u1.id, u2.id] {
        let row = sqlx::query!("SELECT status FROM auth.users WHERE id = $1", uid)
            .fetch_optional(&pool)
            .await
            .expect("query")
            .unwrap();
        assert_eq!(row.status, "suspended_permanent");
    }
}

#[tokio::test]
async fn test_bulk_suspend_given_temp_when_suspend_then_no_purge() {
    let (app, pool) = setup().await;
    let admin = seed_admin(&pool, "bs_temp").await;
    let u = seed_user(&pool, "bs_temp_user").await;
    // Seed dokumen KYC untuk memastikan purge TIDAK terpicu saat sementara.
    // Gunakan status "approved" agar akun menjadi Active (can_transition_to SuspendedTemp valid).
    seed_kyc_submission(&pool, &u, "approved", true).await;

    let resp = app
        .oneshot(post_json_authed(
            "/api/v1/auth/admin/users/suspend",
            &admin.access_token,
            serde_json::json!({
                "user_ids": [u.id],
                "permanent": false,
                "reason": "Penangguhan sementara untuk investigasi",
                "expires_at": "2027-01-01T00:00:00Z",
                "evidence_object_key": "uploads/suspension-evidence/evidence_temp.pdf"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_json(resp).await;
    assert_eq!(body["data"]["results"][0]["success"], true);

    // Verifikasi status sementara
    let row = sqlx::query!("SELECT status FROM auth.users WHERE id = $1", u.id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.status, "suspended_temp");

    // Dokumen TETAP ada (tidak di-purge karena bukan permanen)
    let doc = sqlx::query!(
        "SELECT ktp_object_key, selfie_object_key FROM user_svc.kyc_submission WHERE profile_id = $1 ORDER BY created_at DESC LIMIT 1",
        u.id
    )
    .fetch_optional(&pool)
    .await
    .unwrap();
    assert!(
        doc.is_some(),
        "submission harus tetap ada setelah temp suspend"
    );
    let d = doc.unwrap();
    assert!(
        d.ktp_object_key.is_some() || d.selfie_object_key.is_some(),
        "dokumen harus tetap ada — tidak di-purge saat suspend sementara"
    );
}

#[tokio::test]
async fn test_bulk_suspend_given_permanent_when_suspend_then_documents_purged() {
    let (app, pool) = setup().await;
    let admin = seed_admin(&pool, "bs_purge").await;
    let u = seed_user(&pool, "bs_purge_user").await;
    // Seed dokumen KYC untuk memastikan purge TERPICU saat permanen (D4, task 5.4).
    let submission_id = seed_kyc_submission(&pool, &u, "approved", true).await;

    // Pra-kondisi: dokumen ada sebelum suspend.
    let before = sqlx::query!(
        "SELECT ktp_object_key, selfie_object_key FROM user_svc.kyc_submission WHERE id = $1",
        submission_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        before.ktp_object_key.is_some() && before.selfie_object_key.is_some(),
        "pra-kondisi: dokumen harus ada sebelum suspend permanen"
    );

    let resp = app
        .oneshot(post_json_authed(
            "/api/v1/auth/admin/users/suspend",
            &admin.access_token,
            serde_json::json!({
                "user_ids": [u.id],
                "permanent": true,
                "reason": "Suspend permanen — dokumen harus dimusnahkan",
                "evidence_object_key": "uploads/suspension-evidence/purge.pdf"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_json(resp).await;
    assert_eq!(body["data"]["results"][0]["success"], true);

    // Dokumen ter-purge: referensi key dikosongkan (idempoten clear_document_keys).
    let after = sqlx::query!(
        "SELECT ktp_object_key, selfie_object_key FROM user_svc.kyc_submission WHERE id = $1",
        submission_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        after.ktp_object_key.is_none() && after.selfie_object_key.is_none(),
        "dokumen KTP & Selfie harus di-purge saat suspend permanen"
    );
}

// ── Partial failure ──────────────────────────────────────────────────────────

#[tokio::test]
async fn test_bulk_suspend_given_partial_invalid_when_suspend_then_partial_success() {
    let (app, pool) = setup().await;
    let admin = seed_admin(&pool, "bs_partial").await;
    let valid_user = seed_user(&pool, "bs_valid").await;
    let nonexistent = uuid::Uuid::now_v7();

    let resp = app
        .oneshot(post_json_authed(
            "/api/v1/auth/admin/users/suspend",
            &admin.access_token,
            serde_json::json!({
                "user_ids": [valid_user.id, nonexistent],
                "permanent": true,
                "reason": "Batch dengan sebagian tidak valid",
                "evidence_object_key": "uploads/suspension-evidence/batch.pdf"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_json(resp).await;
    let results = body["data"]["results"].as_array().unwrap();
    assert_eq!(results.len(), 2);

    // Valid user: sukses
    assert_eq!(results[0]["success"], true);
    // Nonexistent user: gagal
    assert_eq!(results[1]["success"], false);
    assert!(!results[1]["error"].as_str().unwrap().is_empty());

    // Verify valid user suspended
    let row = sqlx::query!("SELECT status FROM auth.users WHERE id = $1", valid_user.id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.status, "suspended_permanent");
}

// ── Validation fail-fast ────────────────────────────────────────────────────

#[tokio::test]
async fn test_bulk_suspend_given_no_reason_when_suspend_then_422() {
    let (app, pool) = setup().await;
    let admin = seed_admin(&pool, "bs_noreason").await;
    let u = seed_user(&pool, "bs_noreason_u").await;

    let resp = app
        .oneshot(post_json_authed(
            "/api/v1/auth/admin/users/suspend",
            &admin.access_token,
            serde_json::json!({
                "user_ids": [u.id],
                "permanent": true,
                "reason": ""
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_bulk_suspend_given_temp_no_expires_when_suspend_then_422() {
    let (app, pool) = setup().await;
    let admin = seed_admin(&pool, "bs_noexp").await;
    let u = seed_user(&pool, "bs_noexp_u").await;

    let resp = app
        .oneshot(post_json_authed(
            "/api/v1/auth/admin/users/suspend",
            &admin.access_token,
            serde_json::json!({
                "user_ids": [u.id],
                "permanent": false,
                "reason": "Harus ada expires_at untuk sementara"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

// ── RBAC ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_bulk_suspend_given_non_admin_when_suspend_then_403() {
    let (app, pool) = setup().await;
    let user = seed_user(&pool, "bs_nonadmin").await;

    let resp = app
        .oneshot(post_json_authed(
            "/api/v1/auth/admin/users/suspend",
            &user.access_token,
            serde_json::json!({
                "user_ids": [uuid::Uuid::now_v7()],
                "permanent": true,
                "reason": "Non-admin should be denied"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_bulk_suspend_given_anon_when_suspend_then_401() {
    let (app, _pool) = setup().await;

    let resp = app
        .oneshot(post_json_anon(
            "/api/v1/auth/admin/users/suspend",
            serde_json::json!({
                "user_ids": [uuid::Uuid::now_v7()],
                "permanent": true,
                "reason": "Anon should be denied"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ── Single endpoint masih berfungsi (regression) ─────────────────────────────

#[tokio::test]
async fn test_single_suspend_given_valid_user_when_suspend_then_200() {
    let (app, pool) = setup().await;
    let admin = seed_admin(&pool, "bs_single").await;
    let u = seed_user(&pool, "bs_single_u").await;

    let resp = app
        .oneshot(post_json_authed(
            &format!("/api/v1/auth/admin/users/{}/suspend", u.id),
            &admin.access_token,
            serde_json::json!({
                "permanent": true,
                "reason": "Single suspend via existing endpoint",
                "evidence_object_key": "uploads/suspension-evidence/single.pdf"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let row = sqlx::query!("SELECT status FROM auth.users WHERE id = $1", u.id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.status, "suspended_permanent");
}
