#[tokio::main]
async fn main() {
    let app_env =
        std::env::var("APP_ENV").expect("APP_ENV tidak di-set (development | production)");
    if app_env == "development" {
        dotenvy::dotenv().ok();
    }
    common_tracing::init_tracing();

    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");
    let pool = sqlx::PgPool::connect(&db_url)
        .await
        .expect("DB connect failed");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Migration failed");

    let app = auth_service::router(pool);
    let port = std::env::var("APP_PORT").unwrap_or_else(|_| "3001".into());
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .unwrap();
    tracing::info!("auth-service (standalone) listening on :{port}");
    axum::serve(listener, app).await.unwrap();
}
