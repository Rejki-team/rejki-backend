// Binary standalone region-service (data referensi, read-only). Siap digunakan
// karena tidak membutuhkan dependency eksternal selain database.
#[tokio::main]
async fn main() {
    let app_env =
        std::env::var("APP_ENV").expect("APP_ENV tidak di-set (development | production)");
    if app_env == "development" {
        dotenvy::dotenv().ok();
    }
    common_tracing::init_tracing();

    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL tidak di-set");
    let pool = sqlx::PgPool::connect(&db_url)
        .await
        .expect("DB connect failed");

    let port = std::env::var("REGION_PORT").unwrap_or_else(|_| "3009".into());
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .unwrap();
    tracing::info!("region-service (standalone) listening on :{port}");
    axum::serve(listener, region_service::router(pool))
        .await
        .unwrap();
}
