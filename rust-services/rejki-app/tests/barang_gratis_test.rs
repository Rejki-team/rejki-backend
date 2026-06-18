//! Integration tests untuk model gratis/donasi iklan barang bekas
//! (extend-barang-bekas-gratis-model).
//!
//! Naming: test_{unit}_given_{cond}_when_{action}_then_{expect}
//!
//! Jalankan:
//!   cargo test --test barang_gratis_test -- --test-threads=1
//!
//! Prerequisite:
//!   - PostgreSQL test DB hidup (DATABASE_URL dari .env.test)
//!   - Migrasi sudah dijalankan (termasuk 20260615000001_gratis_model.up.sql)
//!   - RSA key pair ada di ./keys/

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

use common::{
    body_json, build_test_app, fixtures::clean_test_data, get_anon, get_authed, seed_user,
    test_pool,
};

async fn setup() -> (axum::Router, sqlx::PgPool) {
    let _ = dotenvy::from_filename(".env.test");
    let pool = test_pool().await;
    clean_test_data(&pool).await;
    let app = build_test_app(pool.clone()).await;
    (app, pool)
}

fn post_authed(uri: &str, token: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn patch_authed(uri: &str, token: &str) -> Request<Body> {
    Request::builder()
        .method("PATCH")
        .uri(uri)
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap()
}

// ── Create ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_create_given_valid_input_when_create_then_201_with_gratis_fields() {
    let (app, pool) = setup().await;
    let user = seed_user(&pool, "brg_create1").await;

    let resp = app
        .oneshot(post_authed(
            "/api/v1/barang",
            &user.access_token,
            serde_json::json!({
                "judul": "Meja Kayu Bekas",
                "deskripsi": "Meja kantor ukuran 120x60 cm, kondisi baik",
                "jenis_barang": "bekas",
                "jumlah": 2,
                "lokasi_pengambilan": "Jl. Sudirman No. 10, Jakarta Pusat",
                "lokasi": "Jakarta Pusat",
                "foto_urls": ["https://example.com/meja.jpg"]
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = body_json(resp).await;
    let data = &body["data"];
    // Service melewati ammonia::clean_text — spasi di-encode jadi &#32;.
    let judul = data["judul"].as_str().unwrap();
    assert!(
        judul.contains("Meja"),
        "judul harus mengandung 'Meja', got '{judul}'"
    );
    assert_eq!(data["jenis_barang"], "bekas");
    assert_eq!(data["jumlah"], 2);
    // Service melewati ammonia::clean_text untuk lokasi_pengambilan juga.
    assert!(
        data["lokasi_pengambilan"].as_str().unwrap().contains("Jl."),
        "lokasi_pengambilan harus mengandung alamat"
    );
    // Tidak ada field jual-beli.
    assert!(data.get("harga").is_none(), "harga tidak boleh ada");
    assert!(data.get("kondisi").is_none(), "kondisi tidak boleh ada");
    assert!(data.get("is_sold").is_none(), "is_sold tidak boleh ada");
    assert_eq!(data["availability_status"], "tersedia");
}

#[tokio::test]
async fn test_create_given_jenis_barang_invalid_when_create_then_422() {
    let (app, pool) = setup().await;
    let user = seed_user(&pool, "brg_create2").await;

    let resp = app
        .oneshot(post_authed(
            "/api/v1/barang",
            &user.access_token,
            serde_json::json!({
                "judul": "Barang Aneh",
                "deskripsi": "Test",
                "jenis_barang": "rusak_parah",
                "jumlah": 1,
                "lokasi_pengambilan": "Di sini"
            }),
        ))
        .await
        .unwrap();
    // Validasi service-side menolak jenis_barang tidak dikenal.
    assert!(resp.status().is_client_error());
}

#[tokio::test]
async fn test_create_given_jumlah_nol_when_create_then_422() {
    let (app, pool) = setup().await;
    let user = seed_user(&pool, "brg_create3").await;

    let resp = app
        .oneshot(post_authed(
            "/api/v1/barang",
            &user.access_token,
            serde_json::json!({
                "judul": "Nol Item",
                "deskripsi": "Harus ditolak",
                "jenis_barang": "baru",
                "jumlah": 0,
                "lokasi_pengambilan": "Entah"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

// ── mark_taken ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_mark_taken_given_owner_when_patch_then_200_and_status_berubah() {
    let (app, pool) = setup().await;
    let user = seed_user(&pool, "brg_taken1").await;

    // Create dulu
    let create_resp = app
        .clone()
        .oneshot(post_authed(
            "/api/v1/barang",
            &user.access_token,
            serde_json::json!({
                "judul": "Akan Diambil",
                "deskripsi": "Test mark_taken",
                "jenis_barang": "baru",
                "jumlah": 3,
                "lokasi_pengambilan": "Bandung"
            }),
        ))
        .await
        .unwrap();
    let created = body_json(create_resp).await;
    let id = created["data"]["id"].as_str().unwrap().to_string();

    // Mark taken
    let resp = app
        .clone()
        .oneshot(patch_authed(
            &format!("/api/v1/barang/{id}/taken"),
            &user.access_token,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Verifikasi availability_status via GET
    let get_resp = app
        .clone()
        .oneshot(get_authed(
            &format!("/api/v1/barang/{id}"),
            &user.access_token,
        ))
        .await
        .unwrap();
    let body = body_json(get_resp).await;
    assert_eq!(body["data"]["availability_status"], "sudah_diambil");
}

#[tokio::test]
async fn test_mark_taken_given_already_taken_when_patch_again_then_404() {
    let (app, pool) = setup().await;
    let user = seed_user(&pool, "brg_taken2").await;

    // Create + mark_taken pertama
    let create_resp = app
        .clone()
        .oneshot(post_authed(
            "/api/v1/barang",
            &user.access_token,
            serde_json::json!({
                "judul": "Sudah Diambil",
                "deskripsi": "Idempotent test",
                "jenis_barang": "bekas",
                "jumlah": 1,
                "lokasi_pengambilan": "Surabaya"
            }),
        ))
        .await
        .unwrap();
    let created = body_json(create_resp).await;
    let id = created["data"]["id"].as_str().unwrap().to_string();

    // Pertama: OK (200)
    let r1 = app
        .clone()
        .oneshot(patch_authed(
            &format!("/api/v1/barang/{id}/taken"),
            &user.access_token,
        ))
        .await
        .unwrap();
    assert_eq!(r1.status(), StatusCode::OK);

    // Kedua: idempoten → Not Found (atomik WHERE availability_status='tersedia' → 0 rows)
    let r2 = app
        .clone()
        .oneshot(patch_authed(
            &format!("/api/v1/barang/{id}/taken"),
            &user.access_token,
        ))
        .await
        .unwrap();
    assert_eq!(r2.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_mark_taken_given_different_user_when_patch_then_404() {
    let (app, pool) = setup().await;
    let owner = seed_user(&pool, "brg_taken3a").await;
    let other = seed_user(&pool, "brg_taken3b").await;

    let create_resp = app
        .clone()
        .oneshot(post_authed(
            "/api/v1/barang",
            &owner.access_token,
            serde_json::json!({
                "judul": "Bukan Punyamu",
                "deskripsi": "Owner test",
                "jenis_barang": "baru",
                "jumlah": 5,
                "lokasi_pengambilan": "Medan"
            }),
        ))
        .await
        .unwrap();
    let created = body_json(create_resp).await;
    let id = created["data"]["id"].as_str().unwrap().to_string();

    // User lain tidak bisa mark_taken
    let resp = app
        .oneshot(patch_authed(
            &format!("/api/v1/barang/{id}/taken"),
            &other.access_token,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ── Listing publik (dengan filter availability) ──────────────────────────────

#[tokio::test]
async fn test_list_given_mixed_availability_when_list_then_only_tersedia_shown() {
    let (app, pool) = setup().await;
    let user = seed_user(&pool, "brg_list1").await;

    // Create satu barang
    let create_resp = app
        .clone()
        .oneshot(post_authed(
            "/api/v1/barang",
            &user.access_token,
            serde_json::json!({
                "judul": "Barang Tersedia",
                "deskripsi": "Test list",
                "jenis_barang": "baru",
                "jumlah": 1,
                "lokasi_pengambilan": "Semarang"
            }),
        ))
        .await
        .unwrap();
    let created = body_json(create_resp).await;
    let id = created["data"]["id"].as_str().unwrap().to_string();

    // Mark taken → tidak muncul di list publik
    app.clone()
        .oneshot(patch_authed(
            &format!("/api/v1/barang/{id}/taken"),
            &user.access_token,
        ))
        .await
        .unwrap();

    // List publik
    let resp = app.oneshot(get_anon("/api/v1/barang")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_json(resp).await;
    let items = body["data"].as_array().unwrap();
    // Tidak boleh ada yang "sudah_diambil" di list publik.
    for it in items {
        assert_ne!(
            it["availability_status"].as_str().unwrap(),
            "sudah_diambil",
            "listing publik tidak boleh menampilkan barang sudah_diambil"
        );
    }
}

// ── RBAC ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_create_given_anon_when_create_then_401() {
    let (app, _pool) = setup().await;
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/barang")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "judul": "Anonym",
                "deskripsi": "Test",
                "jenis_barang": "bekas",
                "jumlah": 1,
                "lokasi_pengambilan": "Entah"
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ── Kolom baru di response ───────────────────────────────────────────────────

#[tokio::test]
async fn test_response_given_create_when_get_then_kolom_gratis_present() {
    let (app, pool) = setup().await;
    let user = seed_user(&pool, "brg_resp1").await;

    let create_resp = app
        .clone()
        .oneshot(post_authed(
            "/api/v1/barang",
            &user.access_token,
            serde_json::json!({
                "judul": "Kolom Lengkap",
                "deskripsi": "Harus lengkap",
                "jenis_barang": "baru",
                "jumlah": 10,
                "lokasi_pengambilan": "Yogyakarta",
                "lokasi": "DIY",
                "foto_urls": ["https://img.test/1.jpg", "https://img.test/2.jpg"]
            }),
        ))
        .await
        .unwrap();
    let created = body_json(create_resp).await;
    let id = created["data"]["id"].as_str().unwrap().to_string();

    let resp = app
        .oneshot(get_authed(
            &format!("/api/v1/barang/{id}"),
            &user.access_token,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_json(resp).await;
    let data = &body["data"];
    // Kolom baru ada
    assert_eq!(data["jenis_barang"], "baru");
    assert_eq!(data["jumlah"], 10);
    assert_eq!(data["lokasi_pengambilan"], "Yogyakarta");
    assert_eq!(data["availability_status"], "tersedia");
    assert_eq!(data["lokasi"], "DIY");
    // Kolom foto_urls
    let urls = data["foto_urls"].as_array().unwrap();
    assert_eq!(urls.len(), 2);
    // Field jual-beli tidak ada
    assert!(data.get("harga").is_none());
    assert!(data.get("kondisi").is_none());
    assert!(data.get("is_sold").is_none());
}
