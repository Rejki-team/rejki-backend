#[tokio::main]
async fn main() {
    // ── 1. Load centralized config ────────────────────────────────────────
    let cfg = common_config::AppConfig::load();
    let is_prod = cfg.is_production();

    // ── 2. Init tracing ───────────────────────────────────────────────────
    common_tracing::init_tracing(&cfg.otel, is_prod);

    // ── 3. Database pool ──────────────────────────────────────────────────
    let pool = sqlx::PgPool::connect(&cfg.database.url)
        .await
        .expect("gagal connect ke PostgreSQL");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Migration failed");

    // ── 4. Build router ───────────────────────────────────────────────────
    let app = auth_service::router(pool);

    let port = cfg.app_port;
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .unwrap();
    tracing::info!("auth-service (standalone) listening on :{port}");
    axum::serve(listener, app).await.unwrap();
}
