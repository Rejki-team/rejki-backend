// Binary standalone report-service. Hanya membutuhkan database.
// Storage dan notifikasi akan Unavailable; notifikasi tidak berfungsi.
#[tokio::main]
async fn main() {
    let cfg = common_config::AppConfig::load();
    let is_prod = cfg.is_production();

    common_tracing::init_tracing(&cfg.otel, is_prod);

    let pool = sqlx::PgPool::connect(&cfg.database.url)
        .await
        .expect("DB connect failed");

    // Standalone: auth dummy, storage & notif tidak tersedia.
    struct DummyAuthClient;
    #[async_trait::async_trait]
    impl auth_service_client::AuthClient for DummyAuthClient {
        async fn validate_token(
            &self,
            _token: &str,
        ) -> Result<auth_service_client::AuthClaims, auth_service_client::AuthClientError> {
            Err(auth_service_client::AuthClientError::InvalidToken)
        }
        async fn get_account_status(
            &self,
            _user_id: uuid::Uuid,
        ) -> Result<auth_service_client::AccountStatus, auth_service_client::AuthClientError>
        {
            Err(auth_service_client::AuthClientError::Unavailable)
        }
        async fn get_account_email(
            &self,
            _user_id: uuid::Uuid,
        ) -> Result<String, auth_service_client::AuthClientError> {
            Err(auth_service_client::AuthClientError::Unavailable)
        }
        async fn set_account_status(
            &self,
            _user_id: uuid::Uuid,
            _status: auth_service_client::AccountStatus,
        ) -> Result<(), auth_service_client::AuthClientError> {
            Err(auth_service_client::AuthClientError::Unavailable)
        }
        async fn list_active_user_ids(
            &self,
        ) -> Result<Vec<uuid::Uuid>, auth_service_client::AuthClientError> {
            Err(auth_service_client::AuthClientError::Unavailable)
        }
    }

    let auth_client: std::sync::Arc<dyn auth_service_client::AuthClient> =
        std::sync::Arc::new(DummyAuthClient);

    // Backward compat: REPORT_PORT override, fallback ke APP_PORT
    let port = std::env::var("REPORT_PORT")
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(cfg.app_port);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .unwrap();
    tracing::info!("report-service (standalone) listening on :{port}");
    axum::serve(
        listener,
        report_service::router(pool, auth_client, None, None, None),
    )
    .await
    .unwrap();
}
