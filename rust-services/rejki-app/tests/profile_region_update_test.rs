/// W3C-11: Integration test untuk validasi chain region_id di user profiles.
///
/// Menguji endpoint PATCH /api/v1/users/me dengan region_id fields.
/// Test via rejki-app sebagai composition root — semua service ter-wire
/// via domain client (AuthInProcessClient, RegionInProcessClient, dll).
///
/// Prerequisite:
///   - PostgreSQL test DB hidup (DATABASE_URL dari .env.test)
///   - RSA key di ./keys/private.pem + ./keys/public.pem
///   - DATA_ENCRYPTION_KEY di environment
mod common;

use axum::body::Body;
use axum::http::Request;
use tower::ServiceExt;

// ── Helper: seed region data (provinsi 11 → Jawa Barat sampai kelurahan) ──────

async fn seed_region_data(pool: &sqlx::PgPool) {
    sqlx::query(
        "INSERT INTO region.province (id, name) VALUES ($1, $2) ON CONFLICT (id) DO NOTHING",
    )
    .bind("11")
    .bind("Jawa Barat")
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO region.regency (id, province_id, name) VALUES ($1, $2, $3) ON CONFLICT (id) DO NOTHING",
    )
    .bind("1101")
    .bind("11")
    .bind("Kab. Bogor")
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO region.district (id, regency_id, name) VALUES ($1, $2, $3) ON CONFLICT (id) DO NOTHING",
    )
    .bind("110101")
    .bind("1101")
    .bind("Cibinong")
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO region.village (id, district_id, name) VALUES ($1, $2, $3) ON CONFLICT (id) DO NOTHING",
    )
    .bind("1101012001")
    .bind("110101")
    .bind("Kelurahan X")
    .execute(pool)
    .await
    .unwrap();
}

/// Helper: PATCH request dengan JSON body + Bearer token.
fn patch_json(uri: &str, token: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method("PATCH")
        .uri(uri)
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

// ── Test: update profile dengan region chain valid ────────────────────────────

#[tokio::test]
async fn test_patch_me_given_valid_region_chain_when_update_then_region_persisted() {
    let pool = common::test_pool().await;
    common::fixtures::clean_test_data(&pool).await;
    seed_region_data(&pool).await;
    let user = common::fixtures::seed_user(&pool, "reg-val-upd").await;
    let app = common::build_test_app(pool).await;

    let body = serde_json::json!({
        "full_name": "Region Test User",
        "province_id": "11",
        "regency_id": "1101",
        "district_id": "110101",
        "village_id": "1101012001"
    });

    let resp = app
        .oneshot(patch_json("/api/v1/users/me", &user.access_token, body))
        .await
        .unwrap();

    assert_eq!(resp.status(), axum::http::StatusCode::OK);

    let json = common::body_json(resp).await;
    assert_eq!(json["success"], true);

    let data = &json["data"];
    assert_eq!(data["province_id"], "11");
    assert_eq!(data["regency_id"], "1101");
    assert_eq!(data["district_id"], "110101");
    assert_eq!(data["village_id"], "1101012001");
    assert!(data["full_name"]
        .as_str()
        .unwrap_or("")
        .contains("Region Test User"));
}

// ── Test: update profile dengan region chain invalid → VALIDATION_ERROR ────────

#[tokio::test]
async fn test_patch_me_given_invalid_region_chain_when_update_then_422() {
    let pool = common::test_pool().await;
    common::fixtures::clean_test_data(&pool).await;
    seed_region_data(&pool).await;
    let user = common::fixtures::seed_user(&pool, "reg-inv-upd").await;
    let app = common::build_test_app(pool).await;

    // regency_id=9999 tidak ada di bawah province_id=11 → chain tidak konsisten
    let body = serde_json::json!({
        "province_id": "11",
        "regency_id": "9999",
        "district_id": "999901",
        "village_id": "9999012001"
    });

    let resp = app
        .oneshot(patch_json("/api/v1/users/me", &user.access_token, body))
        .await
        .unwrap();

    assert_eq!(resp.status(), axum::http::StatusCode::UNPROCESSABLE_ENTITY);

    let json = common::body_json(resp).await;
    assert_eq!(json["error"], "VALIDATION_ERROR");
    let msg = json["message"].as_str().unwrap_or("").to_lowercase();
    assert!(
        msg.contains("tidak konsisten"),
        "error harus bilang rantai tidak konsisten, dapat: {msg}"
    );
}

// ── Test: update profile dengan region fields partial → VALIDATION_ERROR ──────

#[tokio::test]
async fn test_patch_me_given_partial_region_fields_when_update_then_422() {
    let pool = common::test_pool().await;
    common::fixtures::clean_test_data(&pool).await;
    seed_region_data(&pool).await;
    let user = common::fixtures::seed_user(&pool, "reg-par-upd").await;
    let app = common::build_test_app(pool).await;

    // Hanya province_id diisi → harus error "lengkap atau kosong"
    let body = serde_json::json!({
        "province_id": "11",
        "regency_id": null,
        "district_id": null,
        "village_id": null
    });

    let resp = app
        .oneshot(patch_json("/api/v1/users/me", &user.access_token, body))
        .await
        .unwrap();

    assert_eq!(resp.status(), axum::http::StatusCode::UNPROCESSABLE_ENTITY);

    let json = common::body_json(resp).await;
    assert_eq!(json["error"], "VALIDATION_ERROR");
    // Pesan bisa "region_id harus diisi lengkap ... atau kosong semua"
    let msg = json["message"].as_str().unwrap_or("").to_lowercase();
    assert!(
        msg.contains("lengkap") || msg.contains("kosong"),
        "error harus bilang region harus lengkap atau kosong, dapat: {msg}"
    );
}

// ── Test: update profile tanpa region fields → tetap sukses ───────────────────

#[tokio::test]
async fn test_patch_me_given_no_region_fields_when_update_then_succeeds() {
    let pool = common::test_pool().await;
    common::fixtures::clean_test_data(&pool).await;
    seed_region_data(&pool).await;
    let user = common::fixtures::seed_user(&pool, "reg-skip-upd").await;
    let app = common::build_test_app(pool).await;

    let body = serde_json::json!({
        "full_name": "No Region Update",
        "bio": "Testing without region fields"
    });

    let resp = app
        .oneshot(patch_json("/api/v1/users/me", &user.access_token, body))
        .await
        .unwrap();

    assert_eq!(resp.status(), axum::http::StatusCode::OK);

    let json = common::body_json(resp).await;
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["full_name"], "No Region Update");
}

// ── Test: PATCH /me tanpa auth → 401 ──────────────────────────────────────────

#[tokio::test]
async fn test_patch_me_given_no_auth_when_patch_then_401() {
    let pool = common::test_pool().await;
    let app = common::build_test_app(pool).await;

    let body = serde_json::json!({ "full_name": "Hacker" });

    let req = Request::builder()
        .method("PATCH")
        .uri("/api/v1/users/me")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), axum::http::StatusCode::UNAUTHORIZED);
}
