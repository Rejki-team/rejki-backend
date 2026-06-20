use sqlx::PgPool;

pub async fn seed_region_data(pool: &PgPool) {
    sqlx::query(
        "INSERT INTO region.province (id, name) VALUES ($1, $2) ON CONFLICT (id) DO NOTHING",
    )
    .bind("11")
    .bind("Jawa Barat")
    .execute(pool)
    .await
    .unwrap();

    sqlx::query("INSERT INTO region.regency (id, province_id, name) VALUES ($1, $2, $3) ON CONFLICT (id) DO NOTHING")
        .bind("1101")
        .bind("11")
        .bind("Kab. Bogor")
        .execute(pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO region.district (id, regency_id, name) VALUES ($1, $2, $3) ON CONFLICT (id) DO NOTHING")
        .bind("110101")
        .bind("1101")
        .bind("Cibinong")
        .execute(pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO region.village (id, district_id, name) VALUES ($1, $2, $3) ON CONFLICT (id) DO NOTHING")
        .bind("1101012001")
        .bind("110101")
        .bind("Kelurahan X")
        .execute(pool)
        .await
        .unwrap();
}
