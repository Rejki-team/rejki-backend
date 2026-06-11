// Binary standalone storage-service (presigned URL). Siap digunakan karena hanya
// membutuhkan MinIO endpoint + credentials (tidak butuh AuthClient).
#[tokio::main]
async fn main() {
    let app_env =
        std::env::var("APP_ENV").expect("APP_ENV tidak di-set (development | production)");
    if app_env == "development" {
        dotenvy::dotenv().ok();
    }
    common_tracing::init_tracing();

    let port = std::env::var("STORAGE_PORT").unwrap_or_else(|_| "3010".into());
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .unwrap();
    tracing::info!("storage-service (standalone) listening on :{port}");
    axum::serve(listener, storage_service::router())
        .await
        .unwrap();
}
