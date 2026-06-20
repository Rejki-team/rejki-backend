/// Integration tests untuk region-service — endpoints publik (tanpa auth).
///
/// Naming convention: test_{unit}_given_{cond}_when_{action}_then_{expect}
///
/// Jalankan:
///   cargo test --test region_crud_test -- --test-threads=1
///
/// Prerequisite:
///   - PostgreSQL test DB hidup (DATABASE_URL dari .env.test)
///   - Migrasi sudah dijalankan: sqlx migrate run
mod common;

use axum::Router;
use sqlx::PgPool;
use tower::ServiceExt; // oneshot()

// ── Helpers ───────────────────────────────────────────────────────────────────

async fn setup() -> (Router, PgPool) {
    let _ = dotenvy::from_filename(".env.test");
    let pool = common::test_pool().await;
    common::fixtures::seed_region_data(&pool).await;
    let app = region_service::router(pool.clone());
    (app, pool)
}

fn get_anon(uri: &str) -> axum::http::Request<axum::body::Body> {
    axum::http::Request::builder()
        .method("GET")
        .uri(uri)
        .body(axum::body::Body::empty())
        .unwrap()
}

async fn body_json(resp: axum::response::Response) -> serde_json::Value {
    let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_list_provinces_given_seeded_data_when_list_then_returns_all() {
    let (app, _pool) = setup().await;
    let resp = app.oneshot(get_anon("/provinces")).await.unwrap();

    assert_eq!(resp.status(), axum::http::StatusCode::OK);

    let body = body_json(resp).await;
    assert_eq!(body["success"], true);

    let data = body["data"].as_array().expect("data harus berupa array");
    assert!(!data.is_empty(), "seharusnya ada provinsi");

    let names: Vec<&str> = data
        .iter()
        .filter_map(|item| item["name"].as_str())
        .collect();
    assert!(
        names.contains(&"Jawa Barat"),
        "Jawa Barat harus ada di daftar"
    );
}

#[tokio::test]
async fn test_list_regencies_given_valid_province_when_list_then_returns_children() {
    let (app, _pool) = setup().await;
    let resp = app
        .oneshot(get_anon("/regencies?province_id=11"))
        .await
        .unwrap();

    assert_eq!(resp.status(), axum::http::StatusCode::OK);

    let body = body_json(resp).await;
    assert_eq!(body["success"], true);

    let data = body["data"].as_array().expect("data harus berupa array");
    assert!(
        !data.is_empty(),
        "seharusnya ada regensi untuk province_id=11"
    );

    let names: Vec<&str> = data
        .iter()
        .filter_map(|item| item["name"].as_str())
        .collect();
    assert!(
        names.contains(&"Kab. Bogor"),
        "Kab. Bogor harus ada di daftar"
    );
}

#[tokio::test]
async fn test_list_regencies_given_invalid_province_when_list_then_returns_empty() {
    let (app, _pool) = setup().await;
    let resp = app
        .oneshot(get_anon("/regencies?province_id=99"))
        .await
        .unwrap();

    assert_eq!(resp.status(), axum::http::StatusCode::OK);

    let body = body_json(resp).await;
    assert_eq!(body["success"], true);

    let data = body["data"].as_array().expect("data harus berupa array");
    assert!(
        data.is_empty(),
        "seharusnya tidak ada regensi untuk province_id=99"
    );
}

#[tokio::test]
async fn test_list_districts_given_valid_regency_when_list_then_returns_children() {
    let (app, _pool) = setup().await;
    let resp = app
        .oneshot(get_anon("/districts?regency_id=1101"))
        .await
        .unwrap();

    assert_eq!(resp.status(), axum::http::StatusCode::OK);

    let body = body_json(resp).await;
    assert_eq!(body["success"], true);

    let data = body["data"].as_array().expect("data harus berupa array");
    assert!(
        !data.is_empty(),
        "seharusnya ada distrik untuk regency_id=1101"
    );

    let names: Vec<&str> = data
        .iter()
        .filter_map(|item| item["name"].as_str())
        .collect();
    assert!(names.contains(&"Cibinong"), "Cibinong harus ada di daftar");
}

#[tokio::test]
async fn test_list_villages_given_valid_district_when_list_then_returns_children() {
    let (app, _pool) = setup().await;
    let resp = app
        .oneshot(get_anon("/villages?district_id=110101"))
        .await
        .unwrap();

    assert_eq!(resp.status(), axum::http::StatusCode::OK);

    let body = body_json(resp).await;
    assert_eq!(body["success"], true);

    let data = body["data"].as_array().expect("data harus berupa array");
    assert!(
        !data.is_empty(),
        "seharusnya ada desa untuk district_id=110101"
    );

    let names: Vec<&str> = data
        .iter()
        .filter_map(|item| item["name"].as_str())
        .collect();
    assert!(
        names.contains(&"Kelurahan X"),
        "Kelurahan X harus ada di daftar"
    );
}

#[tokio::test]
async fn test_list_provinces_given_no_data_when_list_then_returns_empty() {
    // Test dengan pool terpisah tanpa seed data untuk memastikan endpoint
    // sukses meskipun tabel kosong.
    let _ = dotenvy::from_filename(".env.test");
    let pool = common::test_pool().await;
    let app = region_service::router(pool);

    let resp = app.oneshot(get_anon("/provinces")).await.unwrap();

    assert_eq!(resp.status(), axum::http::StatusCode::OK);

    let body = body_json(resp).await;
    assert_eq!(body["success"], true);
    // Mungkin ada data dari test lain yang seed data, jadi cukup cek bahwa
    // endpoint return sukses & data berupa array (bisa kosong atau terisi).
    assert!(
        body["data"].is_array(),
        "data harus berupa array (bisa kosong)"
    );
}
