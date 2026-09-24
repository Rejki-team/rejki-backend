//! Integration tests untuk badge/sertifikat iklan pelatihan (P3.2/P3.3, F-10).
//!
//! Naming: test_{unit}_given_{cond}_when_{action}_then_{expect}
//!
//! Jalankan:
//!   cargo test --test pelatihan_badge_test -- --test-threads=1

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

/// P3.2/P3.3 (F-10): admin detail badge tetap 200 + `sertifikat_read_url: null`
/// saat storage-service tidak tersedia (fail-open, pola sama enrollment).
#[tokio::test]
async fn test_admin_badge_detail_given_sertifikat_set_when_get_then_200_with_null_read_url() {
    let (app, pool) = setup().await;
    let admin = seed_admin(&pool, "bdg1a").await;
    let user = seed_user(&pool, "bdg1b").await;
    let iklan_id = seed_iklan_pelatihan(&pool, admin.id, "bdg1", "admin", "default").await;

    let resp = app
        .clone()
        .oneshot(post_authed(
            &format!("/api/v1/pelatihan/{iklan_id}/badge"),
            &user.access_token,
            serde_json::json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let badge_id = body_json(resp).await["data"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let resp = app
        .clone()
        .oneshot(post_authed(
            &format!("/api/v1/pelatihan/badges/{badge_id}/commit-sertifikat"),
            &user.access_token,
            serde_json::json!({ "object_key": "pelatihan-sertifikat/2026/09/test.jpg" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = app
        .clone()
        .oneshot(get_authed(
            &format!("/api/v1/pelatihan/admin/badges/{badge_id}"),
            &admin.access_token,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(
        body["data"]["sertifikat_object_key"],
        "pelatihan-sertifikat/2026/09/test.jpg"
    );
    assert!(body["data"]["sertifikat_read_url"].is_null());
}
