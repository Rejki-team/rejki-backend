//! Integration tests untuk enrollment iklan pelatihan.
//!
//! Naming: test_{unit}_given_{cond}_when_{action}_then_{expect}
//!
//! Jalankan:
//!   cargo test --test pelatihan_enrollment_test -- --test-threads=1

mod common;

use axum::http::StatusCode;
use tower::ServiceExt;

use common::{
    body_json, build_test_app, fixtures::clean_test_data, get_authed, post_authed, seed_admin,
    seed_iklan_pelatihan, seed_user, test_pool,
};

async fn setup() -> (axum::Router, sqlx::PgPool) {
    let _ = dotenvy::from_filename(".env.test");
    let pool = test_pool().await;
    clean_test_data(&pool).await;
    let app = build_test_app(pool.clone()).await;
    (app, pool)
}

#[tokio::test]
async fn test_enroll_given_valid_program_when_enroll_then_201() {
    let (app, pool) = setup().await;
    let admin = seed_admin(&pool, "enr1a").await;
    let user = seed_user(&pool, "enr1b").await;

    // Admin create program → langsung verifikasi_diterima
    let iklan_id = seed_iklan_pelatihan(&pool, admin.id, "enr1", "admin", "default").await;

    // User enroll
    let resp = app
        .clone()
        .oneshot(post_authed(
            &format!("/api/v1/pelatihan/{iklan_id}/enroll"),
            &user.access_token,
            serde_json::json!({}),
        ))
        .await
        .unwrap();
    // create_enrollment returns 201 CREATED
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = body_json(resp).await;
    assert_eq!(body["success"], true);
    assert_eq!(body["data"]["user_id"], user.id.to_string());
}

#[tokio::test]
async fn test_enroll_given_anon_when_enroll_then_401() {
    let (app, _pool) = setup().await;

    use axum::body::Body;
    use axum::http::Request;

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/pelatihan/00000000-0000-0000-0000-000000000000/enroll")
        .header("content-type", "application/json")
        .body(Body::from("{}"))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_enroll_given_nonexistent_program_when_enroll_then_404() {
    let (app, pool) = setup().await;
    let user = seed_user(&pool, "enr2").await;
    let fake_id = uuid::Uuid::now_v7();

    let resp = app
        .clone()
        .oneshot(post_authed(
            &format!("/api/v1/pelatihan/{fake_id}/enroll"),
            &user.access_token,
            serde_json::json!({}),
        ))
        .await
        .unwrap();
    // Program tidak ditemukan
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

/// P3.1/P3.3 (F-10): admin detail tetap 200 + `bukti_transfer_read_url: null`
/// saat storage-service tidak tersedia (fail-open, test harness native tidak
/// pernah wire `StorageClient` — pola sama `report-service`, gap pre-existing
/// dicatat, bukan baru). Membuktikan degradasi anggun, bukan crash.
#[tokio::test]
async fn test_admin_enrollment_detail_given_bukti_set_when_get_then_200_with_null_read_url() {
    let (app, pool) = setup().await;
    let admin = seed_admin(&pool, "enr3a").await;
    let user = seed_user(&pool, "enr3b").await;
    let iklan_id = seed_iklan_pelatihan(&pool, admin.id, "enr3", "admin", "default").await;

    let resp = app
        .clone()
        .oneshot(post_authed(
            &format!("/api/v1/pelatihan/{iklan_id}/enroll"),
            &user.access_token,
            serde_json::json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let enrollment_id = body_json(resp).await["data"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let resp = app
        .clone()
        .oneshot(post_authed(
            &format!("/api/v1/pelatihan/enrollments/{enrollment_id}/commit-bukti"),
            &user.access_token,
            serde_json::json!({ "object_key": "pelatihan-bukti/2026/09/test.jpg" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = app
        .clone()
        .oneshot(get_authed(
            &format!("/api/v1/pelatihan/admin/enrollments/{enrollment_id}"),
            &admin.access_token,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(
        body["data"]["bukti_transfer_object_key"],
        "pelatihan-bukti/2026/09/test.jpg"
    );
    assert!(body["data"]["bukti_transfer_read_url"].is_null());
}
