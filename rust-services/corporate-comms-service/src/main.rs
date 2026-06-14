// Binary standalone corporate-comms-service. Hanya membutuhkan database.
// Storage dan notifikasi akan Unavailable; broadcast tidak berfungsi.
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

    let port = std::env::var("COMMS_PORT").unwrap_or_else(|_| "3014".into());
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .unwrap();
    tracing::info!("corporate-comms-service (standalone) listening on :{port}");
    axum::serve(
        listener,
        corporate_comms_service::router(pool, auth_client, None, None),
    )
    .await
    .unwrap();
}
