//! Integration tests untuk Pengelolaan Pengguna admin (add-user-admin-management).
//!
//! Naming: test_{unit}_given_{cond}_when_{action}_then_{expect}
//!
//! Jalankan:
//!   cargo test --test user_admin_kyc_test -- --test-threads=1
//!
//! Prerequisite:
//!   - PostgreSQL test DB hidup (DATABASE_URL dari .env.test)
//!   - Migrasi semua service sudah dijalankan
//!   - RSA key pair ada di ./keys/
//!
//! Catatan storage: di lingkungan test MinIO biasanya tidak dikonfigurasi, sehingga
//! penerbitan presigned URL mengembalikan "unavailable". Test akses dokumen karena
//! itu memverifikasi perilaku yang TIDAK bergantung pada MinIO: pencatatan audit
//! (dicatat SEBELUM storage), penolakan kind invalid (422), dan keadaan
//! tidak-tersedia saat dokumen sudah dimusnahkan (404).

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt; // oneshot()

use common::{
    body_json, build_test_app, fixtures::clean_test_data, get_anon, get_authed, seed_admin,
    seed_kyc_submission, seed_user, test_pool,
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

// ── Listing ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_admin_list_given_pending_submission_when_list_then_masked_nik_and_meta() {
    let (app, pool) = setup().await;
    let admin = seed_admin(&pool, "list1").await;
    let user = seed_user(&pool, "applicant1").await;
    seed_kyc_submission(&pool, &user, "pending", true).await;

    let resp = app
        .oneshot(get_authed(
            "/api/v1/users/admin/kyc?status=pending",
            &admin.access_token,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_json(resp).await;
    assert_eq!(body["success"], true);
    // Meta paginasi hadir.
    assert!(body["meta"]["total"].as_i64().unwrap() >= 1);
    assert_eq!(body["meta"]["page"], 1);

    let items = body["data"].as_array().expect("data array");
    assert!(!items.is_empty());
    let first = &items[0];
    // NIK hanya ter-mask, tidak pernah penuh.
    assert_eq!(first["nik_masked"], "xxx...8901");
    assert!(
        first.get("nik").is_none(),
        "NIK penuh tidak boleh muncul di listing"
    );
    // Kolom User Story tersedia.
    assert_eq!(first["status"], "pending");
    assert_eq!(first["education_level"], "s1");
    assert_eq!(first["village_id"], "1101012001");
}

#[tokio::test]
async fn test_admin_list_given_search_query_when_list_then_only_matching() {
    let (app, pool) = setup().await;
    let admin = seed_admin(&pool, "list2").await;
    let user = seed_user(&pool, "applicant2").await;
    seed_kyc_submission(&pool, &user, "pending", false).await;

    // Cari berdasarkan ID profil (cocok) — harus mengembalikan minimal 1.
    let uri = format!("/api/v1/users/admin/kyc?status=pending&q={}", user.id);
    let resp = app
        .oneshot(get_authed(&uri, &admin.access_token))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let items = body["data"].as_array().unwrap();
    assert!(items.iter().any(|it| it["nik_masked"] == "xxx...8901"));
}

// ── RBAC ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_admin_endpoints_given_non_admin_when_access_then_403() {
    let (app, pool) = setup().await;
    let user = seed_user(&pool, "nonadmin").await;
    let sub_id = seed_kyc_submission(&pool, &user, "pending", true).await;

    let paths = vec![
        "/api/v1/users/admin/kyc".to_string(),
        "/api/v1/users/admin/kyc/export.csv".to_string(),
        format!("/api/v1/users/admin/kyc/{sub_id}"),
        format!("/api/v1/users/admin/kyc/{sub_id}/documents/ktp"),
    ];
    for p in paths {
        let resp = app
            .clone()
            .oneshot(get_authed(&p, &user.access_token))
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "path {p} harus 403 untuk non-admin"
        );
    }
}

#[tokio::test]
async fn test_admin_list_given_anon_when_access_then_401() {
    let (app, _pool) = setup().await;
    let resp = app
        .oneshot(get_anon("/api/v1/users/admin/kyc"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ── Detail ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_admin_detail_given_existing_when_get_then_masked_detail() {
    let (app, pool) = setup().await;
    let admin = seed_admin(&pool, "detail1").await;
    let user = seed_user(&pool, "applicant3").await;
    let sub_id = seed_kyc_submission(&pool, &user, "pending", true).await;

    let resp = app
        .oneshot(get_authed(
            &format!("/api/v1/users/admin/kyc/{sub_id}"),
            &admin.access_token,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["data"]["id"], sub_id.to_string());
    assert_eq!(body["data"]["nik_masked"], "xxx...8901");
    assert_eq!(body["data"]["has_ktp"], true);
    assert_eq!(body["data"]["has_selfie"], true);
}

#[tokio::test]
async fn test_admin_detail_given_unknown_id_when_get_then_404() {
    let (app, pool) = setup().await;
    let admin = seed_admin(&pool, "detail2").await;
    let unknown = uuid::Uuid::now_v7();

    let resp = app
        .oneshot(get_authed(
            &format!("/api/v1/users/admin/kyc/{unknown}"),
            &admin.access_token,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ── Dokumen (audit + validasi + tidak-tersedia) ──────────────────────────────

#[tokio::test]
async fn test_admin_document_given_invalid_kind_when_get_then_422() {
    let (app, pool) = setup().await;
    let admin = seed_admin(&pool, "doc1").await;
    let user = seed_user(&pool, "applicant4").await;
    let sub_id = seed_kyc_submission(&pool, &user, "pending", true).await;

    let resp = app
        .oneshot(get_authed(
            &format!("/api/v1/users/admin/kyc/{sub_id}/documents/passport"),
            &admin.access_token,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_admin_document_given_purged_when_get_then_404_unavailable() {
    let (app, pool) = setup().await;
    let admin = seed_admin(&pool, "doc2").await;
    let user = seed_user(&pool, "applicant5").await;
    // with_documents=false → object key NULL (seolah sudah dimusnahkan).
    let sub_id = seed_kyc_submission(&pool, &user, "rejected", false).await;

    let resp = app
        .oneshot(get_authed(
            &format!("/api/v1/users/admin/kyc/{sub_id}/documents/ktp"),
            &admin.access_token,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_admin_document_given_request_when_get_then_audit_recorded_with_admin_actor() {
    let (app, pool) = setup().await;
    let admin = seed_admin(&pool, "doc3").await;
    let user = seed_user(&pool, "applicant6").await;
    let sub_id = seed_kyc_submission(&pool, &user, "pending", true).await;

    // Akses dokumen KTP. Audit dicatat SEBELUM penerbitan URL; status akhir
    // bergantung MinIO (200 bila ada, 500 unavailable bila tidak) — yang diuji
    // di sini adalah pencatatan audit, bukan URL.
    let _ = app
        .oneshot(get_authed(
            &format!("/api/v1/users/admin/kyc/{sub_id}/documents/ktp"),
            &admin.access_token,
        ))
        .await
        .unwrap();

    let expected_key = format!("uploads/ktp/{}/ktp.jpg", user.id);
    let row = sqlx::query!(
        "SELECT actor_id, action FROM user_svc.document_access_log \
         WHERE object_key = $1 AND action = 'read_issued'",
        expected_key,
    )
    .fetch_optional(&pool)
    .await
    .expect("query audit");

    let row = row.expect("audit read_issued harus tercatat");
    // Aktor = admin (akuntabilitas), bukan pemilik dokumen.
    assert_eq!(row.actor_id, admin.id);
}

// ── Review: idempotensi & auto-purge ─────────────────────────────────────────

#[tokio::test]
async fn test_review_given_terminal_submission_when_review_again_then_409() {
    let (app, pool) = setup().await;
    let admin = seed_admin(&pool, "rev1").await;
    let user = seed_user(&pool, "applicant7").await;
    // Sudah approved (terminal).
    let sub_id = seed_kyc_submission(&pool, &user, "approved", false).await;

    let resp = app
        .oneshot(post_authed(
            &format!("/api/v1/users/admin/kyc/{sub_id}/review"),
            &admin.access_token,
            serde_json::json!({ "approved": false, "review_note": "ulang" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn test_review_given_reject_when_review_then_documents_purged() {
    let (app, pool) = setup().await;
    let admin = seed_admin(&pool, "rev2").await;
    let user = seed_user(&pool, "applicant8").await;
    let sub_id = seed_kyc_submission(&pool, &user, "pending", true).await;

    let resp = app
        .oneshot(post_authed(
            &format!("/api/v1/users/admin/kyc/{sub_id}/review"),
            &admin.access_token,
            serde_json::json!({ "approved": false, "review_note": "data tidak valid" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Referensi dokumen dikosongkan (auto-purge best-effort tetap clear keys).
    let row = sqlx::query!(
        "SELECT ktp_object_key, selfie_object_key, status \
         FROM user_svc.kyc_submission WHERE id = $1",
        sub_id,
    )
    .fetch_one(&pool)
    .await
    .expect("query submission");

    assert_eq!(row.status, "rejected");
    assert!(
        row.ktp_object_key.is_none(),
        "ktp_object_key harus NULL setelah purge"
    );
    assert!(
        row.selfie_object_key.is_none(),
        "selfie_object_key harus NULL setelah purge"
    );
}

// ── CSV Export ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_admin_export_csv_given_submission_when_export_then_csv_with_headers() {
    let (app, pool) = setup().await;
    let admin = seed_admin(&pool, "csv1").await;
    let user = seed_user(&pool, "applicant9").await;
    seed_kyc_submission(&pool, &user, "pending", false).await;

    let resp = app
        .oneshot(get_authed(
            "/api/v1/users/admin/kyc/export.csv?status=pending",
            &admin.access_token,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Verifikasi header CSV dan content type.
    let headers = resp.headers();
    assert!(headers
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap()
        .contains("text/csv"));
    assert!(headers
        .get("content-disposition")
        .unwrap()
        .to_str()
        .unwrap()
        .contains("kyc_submissions.csv"));

    let body = String::from_utf8(
        axum::body::to_bytes(resp.into_body(), 1024 * 1024)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();

    assert!(body.contains("ID,Nama,Pendidikan,Gender,Tanggal_Lahir,Alamat"));
    assert!(body.contains("Budi Tester"));
    assert!(body.contains("s1"));
    assert!(
        !body.contains("8901"),
        "CSV tidak boleh mengandung NIK penuh/asli"
    );
}

#[tokio::test]
async fn test_admin_export_csv_given_non_admin_when_export_then_403() {
    let (app, pool) = setup().await;
    let user = seed_user(&pool, "csvnonadmin").await;

    let resp = app
        .oneshot(get_authed(
            "/api/v1/users/admin/kyc/export.csv",
            &user.access_token,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}
