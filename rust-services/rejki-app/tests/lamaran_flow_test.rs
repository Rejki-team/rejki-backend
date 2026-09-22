//! Integration test alur Lamaran Pekerjaan (F-3, Kelompok 3 Phase 1).
//!
//! Naming: test_{unit}_given_{cond}_when_{action}_then_{expect}
//!
//! Jalankan:
//!   cargo test --test lamaran_flow_test -- --test-threads=1
//!
//! Prerequisite: sama seperti auth_integration_test.rs (PostgreSQL test DB + migrasi +
//! RSA key di ./keys/).

mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use tower::ServiceExt; // oneshot()
use uuid::Uuid;

async fn setup() -> (axum::Router, sqlx::PgPool) {
    let _ = dotenvy::from_filename(".env.test");
    let pool = common::test_pool().await;
    common::fixtures::clean_test_data(&pool).await;
    let app = common::build_test_app(pool.clone()).await;
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

fn patch_authed(uri: &str, token: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method("PATCH")
        .uri(uri)
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

/// Buat 1 Iklan Pekerjaan milik `owner`, lalu set koordinat langsung via SQL (bypass
/// geocoding — tidak ada `geocoding_client` di test harness) agar validasi geofence
/// (P1.5) dapat diuji deterministik.
async fn seed_iklan_pekerjaan_with_coords(
    app: &axum::Router,
    pool: &sqlx::PgPool,
    owner_token: &str,
    lat: f64,
    lng: f64,
) -> Uuid {
    let resp = app
        .clone()
        .oneshot(post_authed(
            "/api/v1/pekerjaan",
            owner_token,
            json!({
                "judul": "Tukang Kebun Harian",
                "perusahaan": "PT Uji Lamaran",
                "deskripsi": "Deskripsi lamaran test yang cukup panjang",
                "tipe": "harian",
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let id: Uuid = common::body_json(resp).await["data"]["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();

    sqlx::query("UPDATE iklan_pekerjaan.iklan SET latitude=$1, longitude=$2 WHERE id=$3")
        .bind(lat)
        .bind(lng)
        .bind(id)
        .execute(pool)
        .await
        .expect("seed: set koordinat iklan gagal");

    id
}

/// Pelamar wajib punya Iklan Pekerja aktif (P1.3 validasi #1) sebelum bisa melamar.
async fn seed_iklan_pekerja(app: &axum::Router, pelamar_token: &str) {
    let resp = app
        .clone()
        .oneshot(post_authed(
            "/api/v1/pekerja",
            pelamar_token,
            json!({
                "nama": "Pekerja Uji",
                "keahlian": ["Berkebun"],
                "deskripsi": "Deskripsi pekerja test",
                "lokasi": "Jakarta",
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
}

const JAKARTA_LAT: f64 = -6.2088;
const JAKARTA_LNG: f64 = 106.8456;
/// >50m dari JAKARTA_LAT/LNG (~diluar radius geofence) — dipakai jika perlu uji gagal.
#[allow(dead_code)]
const FAR_LAT: f64 = -6.9175;
#[allow(dead_code)]
const FAR_LNG: f64 = 107.6191;

/// P1.9: alur penuh Diajukan→Diterima→Proses(geofence)→Selesai.
#[tokio::test]
async fn test_lamaran_given_full_flow_when_diajukan_to_selesai_then_status_transitions_correctly() {
    let (app, pool) = setup().await;
    let owner = common::seed_user(&pool, "lamflow-owner").await;
    let pelamar = common::seed_user(&pool, "lamflow-pelamar").await;

    let iklan_id = seed_iklan_pekerjaan_with_coords(
        &app,
        &pool,
        &owner.access_token,
        JAKARTA_LAT,
        JAKARTA_LNG,
    )
    .await;
    seed_iklan_pekerja(&app, &pelamar.access_token).await;

    // 1. Ajukan lamaran (Diajukan).
    let resp = app
        .clone()
        .oneshot(post_authed(
            &format!("/api/v1/pekerjaan/{iklan_id}/lamar"),
            &pelamar.access_token,
            json!({
                "tanggal": "2026-11-01",
                "jam_mulai": "08:00:00",
                "jam_akhir": "16:00:00",
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = common::body_json(resp).await;
    assert_eq!(body["data"]["status"], "diajukan");
    let lamaran_id: Uuid = body["data"]["id"].as_str().unwrap().parse().unwrap();

    // 2. Owner menerima (Diterima).
    let resp = app
        .clone()
        .oneshot(patch_authed(
            &format!("/api/v1/pekerjaan/{iklan_id}/lamaran/{lamaran_id}"),
            &owner.access_token,
            json!({"approved": true}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(common::body_json(resp).await["data"]["status"], "diterima");

    // 3. Pelamar mulai bekerja — geofence lokasi sama dengan iklan (Proses).
    let resp = app
        .clone()
        .oneshot(post_authed(
            &format!("/api/v1/pekerjaan/{iklan_id}/lamaran/{lamaran_id}/mulai-bekerja"),
            &pelamar.access_token,
            json!({"latitude": JAKARTA_LAT, "longitude": JAKARTA_LNG}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(common::body_json(resp).await["data"]["status"], "proses");

    // Iklan ikut berubah SedangDikerjakan (transaksi atomik Hazard #4).
    let resp = app
        .clone()
        .oneshot(common::get_anon(&format!("/api/v1/pekerjaan/{iklan_id}")))
        .await
        .unwrap();
    assert_eq!(
        common::body_json(resp).await["data"]["status"],
        "sedang_dikerjakan"
    );

    // P6.9 (Kelompok 2 Bab 10): Sedang Dikerjakan → sembunyikan dari daftar publik.
    // Detail (di atas) tetap bisa diakses, hanya listing publik yang menyaring.
    let resp = app
        .clone()
        .oneshot(common::get_anon("/api/v1/pekerjaan?limit=50"))
        .await
        .unwrap();
    let ids: Vec<String> = common::body_json(resp).await["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["id"].as_str().unwrap().to_string())
        .collect();
    assert!(
        !ids.contains(&iklan_id.to_string()),
        "iklan sedang_dikerjakan tidak boleh muncul di daftar publik"
    );

    // 4. Pelamar tandai selesai (Selesai).
    let resp = app
        .clone()
        .oneshot(post_authed(
            &format!("/api/v1/pekerjaan/{iklan_id}/lamaran/{lamaran_id}/tandai-selesai"),
            &pelamar.access_token,
            json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(common::body_json(resp).await["data"]["status"], "selesai");

    let resp = app
        .oneshot(common::get_anon(&format!("/api/v1/pekerjaan/{iklan_id}")))
        .await
        .unwrap();
    assert_eq!(common::body_json(resp).await["data"]["status"], "selesai");
}

/// P1.9: melamar dengan rentang jam yang bentrok dengan lamaran aktif (Diterima) → ditolak.
#[tokio::test]
async fn test_lamar_given_conflicting_schedule_when_lamar_then_returns_conflict() {
    let (app, pool) = setup().await;
    let owner_a = common::seed_user(&pool, "lamconflict-ownera").await;
    let owner_b = common::seed_user(&pool, "lamconflict-ownerb").await;
    let pelamar = common::seed_user(&pool, "lamconflict-pelamar").await;

    let iklan_a = seed_iklan_pekerjaan_with_coords(
        &app,
        &pool,
        &owner_a.access_token,
        JAKARTA_LAT,
        JAKARTA_LNG,
    )
    .await;
    let iklan_b = seed_iklan_pekerjaan_with_coords(
        &app,
        &pool,
        &owner_b.access_token,
        JAKARTA_LAT,
        JAKARTA_LNG,
    )
    .await;
    seed_iklan_pekerja(&app, &pelamar.access_token).await;

    // Lamaran pertama, diterima owner_a.
    let resp = app
        .clone()
        .oneshot(post_authed(
            &format!("/api/v1/pekerjaan/{iklan_a}/lamar"),
            &pelamar.access_token,
            json!({"tanggal": "2026-11-05", "jam_mulai": "08:00:00", "jam_akhir": "16:00:00"}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let lamaran_a_id: Uuid = common::body_json(resp).await["data"]["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let resp = app
        .clone()
        .oneshot(patch_authed(
            &format!("/api/v1/pekerjaan/{iklan_a}/lamaran/{lamaran_a_id}"),
            &owner_a.access_token,
            json!({"approved": true}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Lamaran kedua, jam tumpang tindih (10:00-12:00 vs 08:00-16:00) di tanggal sama → 409.
    let resp = app
        .oneshot(post_authed(
            &format!("/api/v1/pekerjaan/{iklan_b}/lamar"),
            &pelamar.access_token,
            json!({"tanggal": "2026-11-05", "jam_mulai": "10:00:00", "jam_akhir": "12:00:00"}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

/// P1.3 validasi #1: pelamar belum punya Iklan Pekerja aktif → ditolak.
#[tokio::test]
async fn test_lamar_given_no_iklan_pekerja_when_lamar_then_returns_validation_error() {
    let (app, pool) = setup().await;
    let owner = common::seed_user(&pool, "lamnopekerja-owner").await;
    let pelamar = common::seed_user(&pool, "lamnopekerja-pelamar").await;

    let iklan_id = seed_iklan_pekerjaan_with_coords(
        &app,
        &pool,
        &owner.access_token,
        JAKARTA_LAT,
        JAKARTA_LNG,
    )
    .await;
    // Pelamar TIDAK di-seed Iklan Pekerja.

    let resp = app
        .oneshot(post_authed(
            &format!("/api/v1/pekerjaan/{iklan_id}/lamar"),
            &pelamar.access_token,
            json!({"tanggal": "2026-11-10", "jam_mulai": "08:00:00", "jam_akhir": "16:00:00"}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

/// P1.8 (IDOR): user lain (bukan pemilik iklan) mencoba terima/tolak lamaran → 404.
#[tokio::test]
async fn test_review_lamaran_given_other_user_as_owner_when_review_then_404() {
    let (app, pool) = setup().await;
    let owner = common::seed_user(&pool, "lamidor-owner").await;
    let attacker = common::seed_user(&pool, "lamidor-attacker").await;
    let pelamar = common::seed_user(&pool, "lamidor-pelamar").await;

    let iklan_id = seed_iklan_pekerjaan_with_coords(
        &app,
        &pool,
        &owner.access_token,
        JAKARTA_LAT,
        JAKARTA_LNG,
    )
    .await;
    seed_iklan_pekerja(&app, &pelamar.access_token).await;

    let resp = app
        .clone()
        .oneshot(post_authed(
            &format!("/api/v1/pekerjaan/{iklan_id}/lamar"),
            &pelamar.access_token,
            json!({"tanggal": "2026-11-12", "jam_mulai": "08:00:00", "jam_akhir": "16:00:00"}),
        ))
        .await
        .unwrap();
    let lamaran_id: Uuid = common::body_json(resp).await["data"]["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();

    // Attacker (bukan owner) coba terima lamaran → 404, bukan 403 (IDOR, §4.4).
    let resp = app
        .clone()
        .oneshot(patch_authed(
            &format!("/api/v1/pekerjaan/{iklan_id}/lamaran/{lamaran_id}"),
            &attacker.access_token,
            json!({"approved": true}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // Attacker coba lihat daftar pelamar iklan orang lain → 404.
    let resp = app
        .oneshot(common::get_authed(
            &format!("/api/v1/pekerjaan/{iklan_id}/lamaran"),
            &attacker.access_token,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

/// P1.8 (IDOR): user lain (bukan pelamar) mencoba mulai-bekerja/tandai-selesai → 404.
#[tokio::test]
async fn test_mulai_bekerja_given_other_user_as_pelamar_when_mulai_then_404() {
    let (app, pool) = setup().await;
    let owner = common::seed_user(&pool, "lamidor2-owner").await;
    let pelamar = common::seed_user(&pool, "lamidor2-pelamar").await;
    let attacker = common::seed_user(&pool, "lamidor2-attacker").await;

    let iklan_id = seed_iklan_pekerjaan_with_coords(
        &app,
        &pool,
        &owner.access_token,
        JAKARTA_LAT,
        JAKARTA_LNG,
    )
    .await;
    seed_iklan_pekerja(&app, &pelamar.access_token).await;

    let resp = app
        .clone()
        .oneshot(post_authed(
            &format!("/api/v1/pekerjaan/{iklan_id}/lamar"),
            &pelamar.access_token,
            json!({"tanggal": "2026-11-14", "jam_mulai": "08:00:00", "jam_akhir": "16:00:00"}),
        ))
        .await
        .unwrap();
    let lamaran_id: Uuid = common::body_json(resp).await["data"]["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();

    let resp = app
        .clone()
        .oneshot(patch_authed(
            &format!("/api/v1/pekerjaan/{iklan_id}/lamaran/{lamaran_id}"),
            &owner.access_token,
            json!({"approved": true}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Attacker (bukan pelamar) coba mulai bekerja → 404.
    let resp = app
        .oneshot(post_authed(
            &format!("/api/v1/pekerjaan/{iklan_id}/lamaran/{lamaran_id}/mulai-bekerja"),
            &attacker.access_token,
            json!({"latitude": JAKARTA_LAT, "longitude": JAKARTA_LNG}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
