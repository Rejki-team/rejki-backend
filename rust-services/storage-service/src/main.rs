// Binary standalone storage-service (presigned URL + scan worker).
#[tokio::main]
async fn main() {
    let cfg = common_config::AppConfig::load();
    let is_prod = cfg.is_production();

    common_tracing::init_tracing(&cfg.otel, is_prod);

    // ── Start scan worker (opsional — fail-open jika ClamAV/Redis tidak ada) ─
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    if let Some(worker) = storage_service::infrastructure::scan_worker::ScanWorker::from_config(
        &cfg.minio,
        &cfg.clamav,
        &cfg.redis.url,
        shutdown_rx,
    )
    .await
    {
        tokio::spawn(async move {
            worker.run().await;
        });
        tracing::info!("scan worker started (ClamAV consumer)");
    } else {
        tracing::warn!("scan worker not started — ClamAV/Redis not configured");
    }

    // Backward compat: STORAGE_PORT override, fallback ke APP_PORT
    let port = std::env::var("STORAGE_PORT")
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(cfg.app_port);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .unwrap();
    tracing::info!("storage-service (standalone) listening on :{port}");
    axum::serve(listener, storage_service::router())
        .with_graceful_shutdown(async move {
            tokio::signal::ctrl_c().await.ok();
            let _ = shutdown_tx.send(true);
        })
        .await
        .unwrap();
}
