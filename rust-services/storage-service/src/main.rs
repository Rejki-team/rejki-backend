// Binary standalone storage-service (presigned URL + scan worker). Siap digunakan karena hanya
// membutuhkan MinIO endpoint + credentials (tidak butuh AuthClient).
#[tokio::main]
async fn main() {
    let app_env =
        std::env::var("APP_ENV").expect("APP_ENV tidak di-set (development | production)");
    if app_env == "development" {
        dotenvy::dotenv().ok();
    }
    common_tracing::init_tracing();

    // ── Start scan worker (opsional — fail-open jika ClamAV/Redis tidak ada) ─
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    if let Some(worker) =
        storage_service::infrastructure::scan_worker::ScanWorker::from_env(shutdown_rx).await
    {
        tokio::spawn(async move {
            worker.run().await;
        });
        tracing::info!("scan worker started (ClamAV consumer)");
    } else {
        tracing::warn!("scan worker not started — ClamAV/Redis not configured");
    }

    let port = std::env::var("STORAGE_PORT").unwrap_or_else(|_| "3010".into());
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
