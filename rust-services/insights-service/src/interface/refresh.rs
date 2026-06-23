use std::sync::Arc;

use crate::application::service::InsightsService;
use crate::infrastructure::pg_repository::PgInsightsRepository;

/// Refresh interval: 30 menit
const REFRESH_INTERVAL_MINUTES: u64 = 30;

/// Auto-refresh coordinator — memperbarui materialized views setiap 30 menit.
/// Error di-log, tidak panic (fail-open).
pub async fn start_auto_refresh(svc: Arc<InsightsService<PgInsightsRepository>>) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(
        REFRESH_INTERVAL_MINUTES * 60,
    ));
    loop {
        interval.tick().await;
        tracing::info!(
            "auto-refresh materialized views (setiap {} menit)",
            REFRESH_INTERVAL_MINUTES
        );
        match svc.refresh().await {
            Ok(resp) => tracing::info!("auto-refresh selesai: {}", resp.message),
            Err(e) => {
                tracing::error!(error = ?e, "auto-refresh gagal — akan coba lagi di siklus berikutnya")
            }
        }
    }
}
