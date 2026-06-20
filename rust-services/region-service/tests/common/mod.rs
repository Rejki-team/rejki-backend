pub mod fixtures;

use sqlx::PgPool;

pub async fn test_pool() -> PgPool {
    let _ = dotenvy::from_filename(".env.test");
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL harus di-set");
    PgPool::connect(&db_url)
        .await
        .expect("gagal connect ke test database")
}
