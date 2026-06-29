#![allow(dead_code)]
use std::sync::Arc;

use axum::{body::Body, http::Request, Router};
use serde_json::Value;
use sqlx::PgPool;

pub mod fixtures;

#[allow(unused_imports)]
pub use fixtures::{seed_admin, seed_kyc_submission, seed_user};

/// Test app — build router yang sama dengan main.rs tapi pakai pool test.
pub async fn build_test_app(pool: PgPool) -> Router {
    let private_pem = std::fs::read_to_string(
        std::env::var("JWT_PRIVATE_KEY_PATH").unwrap_or_else(|_| "./keys/private.pem".into()),
    )
    .expect("JWT_PRIVATE_KEY_PATH not found");

    let public_pem = std::fs::read_to_string(
        std::env::var("JWT_PUBLIC_KEY_PATH").unwrap_or_else(|_| "./keys/public.pem".into()),
    )
    .expect("JWT_PUBLIC_KEY_PATH not found");

    let jwt = Arc::new(
        auth_service::JwtService::from_files(&private_pem, &public_pem, 900)
            .expect("JwtService init failed"),
    );

    let auth_repo = Arc::new(auth_service::PgAuthRepository::new(pool.clone()));
    let auth_client: Arc<dyn auth_service::AuthClient> = Arc::new(
        auth_service::AuthInProcessClient::new(jwt.clone(), auth_repo.clone()),
    );

    use axum::response::Json;
    use axum::routing::get;
    use serde_json::json;

    // Shared clients — dibangun sekali agar test mencerminkan komposisi main.rs.
    let region_client: Arc<dyn region_service::RegionClient> = Arc::new(
        region_service::RegionInProcessClient::new(Arc::new(region_service::RegionService::new(
            Arc::new(region_service::PgRegionRepository::new(pool.clone())),
        ))),
    );
    let storage_client: Arc<dyn storage_service::StorageClient> =
        Arc::new(storage_service::StorageInProcessClient::new().await);

    // UserClient in-process — wajib agar jalur purge dokumen saat suspend permanen
    // (extend-user-suspension-bulk-purge D4) ter-cover di integration test.
    let user_client: Arc<dyn user_service::UserClient> = {
        let user_repo = Arc::new(user_service::PgUserRepository::new(pool.clone()));
        let user_svc = Arc::new(user_service::UserService::new(
            user_repo,
            auth_client.clone(),
            region_client.clone(),
            Some(storage_client.clone()),
            None,
        ));
        Arc::new(user_service::UserInProcessClient::new(user_svc))
    };

    let api_v1 = Router::new()
        .nest(
            "/auth",
            auth_service::router_with_deps_ex(
                jwt.clone(),
                auth_repo.clone(),
                auth_client.clone(),
                None,
                Some(storage_client.clone()),
                Some(user_client.clone()),
                900,
                None,
            ),
        )
        .nest(
            "/users",
            user_service::router(
                pool.clone(),
                auth_client.clone(),
                region_client.clone(),
                storage_client.clone(),
                None,
            ),
        )
        .nest(
            "/chat",
            chat_service::router(pool.clone(), auth_client.clone(), None),
        )
        .nest(
            "/notif",
            notification_service::router(pool.clone(), auth_client.clone(), None),
        )
        .nest(
            "/pekerjaan",
            iklan_pekerjaan_service::router(
                pool.clone(),
                auth_client.clone(),
                None,
                None,
                None,
                None,
            ),
        )
        .nest(
            "/pekerja",
            iklan_pekerja_service::router(
                pool.clone(),
                auth_client.clone(),
                None,
                None,
                None,
                None,
            ),
        )
        .nest(
            "/barang",
            iklan_barang_bekas_service::router(
                pool.clone(),
                auth_client.clone(),
                None,
                None,
                None,
                None,
            ),
        )
        .nest(
            "/pelatihan",
            iklan_pelatihan_service::router(
                pool.clone(),
                auth_client.clone(),
                None,
                None,
                None,
                None,
            ),
        );

    Router::new()
        .route("/health", get(|| async { Json(json!({"status": "ok"})) }))
        .nest("/api/v1", api_v1)
        .layer(axum::middleware::from_fn(common_tracing::request_id_layer))
}

/// Koneksi ke database test. Panic jika DATABASE_URL tidak di-set.
pub async fn test_pool() -> PgPool {
    // Load .env.test jika belum ada env var
    let _ = dotenvy::from_filename(".env.test");

    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL harus di-set di .env.test");

    sqlx::PgPool::connect(&db_url)
        .await
        .expect("gagal connect ke test database")
}

/// Helper: POST request dengan JSON body, tanpa auth header.
#[allow(dead_code)]
pub fn post_json(uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

/// Helper: GET request dengan optional Bearer token.
pub fn get_authed(uri: &str, token: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(uri)
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap()
}

/// Helper: GET request tanpa auth.
pub fn get_anon(uri: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

/// Helper: Baca response body sebagai JSON.
pub async fn body_json(resp: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
}
