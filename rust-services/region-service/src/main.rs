// Binary standalone region-service (data referensi, read-only). Siap digunakan
// karena tidak membutuhkan dependency eksternal selain database.
#[tokio::main]
async fn main() {
    let cfg = common_config::AppConfig::load();
    let is_prod = cfg.is_production();

    common_tracing::init_tracing(&cfg.otel, is_prod);

    let pool = sqlx::PgPool::connect(&cfg.database.url)
        .await
        .expect("DB connect failed");

    // Backward compat: REGION_PORT override, fallback ke APP_PORT
    let port = std::env::var("REGION_PORT")
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(cfg.app_port);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .unwrap();
    tracing::info!("region-service (standalone) listening on :{port}");
    axum::serve(listener, region_service::router(pool))
        .await
        .unwrap();
}
