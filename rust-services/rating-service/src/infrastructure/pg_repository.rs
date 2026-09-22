use std::time::Instant;

use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::domain::entity::{Rating, RatingAggregate, RatingArah};
use crate::domain::repository::{CreateRatingParams, RatingRepository};

pub struct PgRatingRepository {
    pool: PgPool,
}

impl PgRatingRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

macro_rules! warn_slow {
    ($start:expr, $op:expr) => {
        let elapsed = $start.elapsed();
        if elapsed.as_millis() > 100 {
            tracing::warn!(op = $op, elapsed_ms = elapsed.as_millis(), "slow DB query");
        }
    };
}

fn row_to_rating(r: &sqlx::postgres::PgRow) -> Rating {
    let arah_raw: String = r.get("arah");
    Rating {
        id: r.get("id"),
        iklan_id: r.get("iklan_id"),
        penilai_id: r.get("penilai_id"),
        dinilai_id: r.get("dinilai_id"),
        arah: RatingArah::parse(&arah_raw).unwrap_or(RatingArah::PelamarKePemberiKerja),
        bintang: r.get("bintang"),
        ulasan: r.get("ulasan"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    }
}

impl RatingRepository for PgRatingRepository {
    async fn create(&self, params: CreateRatingParams<'_>) -> Result<Rating, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query(SQL_CREATE)
            .bind(params.iklan_id)
            .bind(params.penilai_id)
            .bind(params.dinilai_id)
            .bind(params.arah.as_str())
            .bind(params.bintang)
            .bind(params.ulasan)
            .fetch_one(&self.pool)
            .await?;
        warn_slow!(t, "rating.create");
        Ok(row_to_rating(&r))
    }

    async fn exists_for_pair(
        &self,
        iklan_id: Uuid,
        penilai_id: Uuid,
        dinilai_id: Uuid,
    ) -> Result<bool, anyhow::Error> {
        let t = Instant::now();
        let r: Option<bool> = sqlx::query_scalar(SQL_EXISTS_FOR_PAIR)
            .bind(iklan_id)
            .bind(penilai_id)
            .bind(dinilai_id)
            .fetch_one(&self.pool)
            .await?;
        warn_slow!(t, "rating.exists_for_pair");
        Ok(r.unwrap_or(false))
    }

    async fn get_aggregate_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<RatingAggregate, anyhow::Error> {
        let t = Instant::now();
        let row = sqlx::query(SQL_AGGREGATE_FOR_USER)
            .bind(user_id)
            .fetch_one(&self.pool)
            .await?;
        warn_slow!(t, "rating.get_aggregate_for_user");
        let average: Option<f64> = row.try_get("average").unwrap_or(None);
        let count: i64 = row.try_get("count").unwrap_or(0);
        Ok(RatingAggregate {
            average: average.unwrap_or(0.0),
            count,
        })
    }
}

// ── SQL literal statis — sqlx anti-dynamic-string ──────────────────────────────
const SQL_CREATE: &str = "INSERT INTO rating.rating (id,iklan_id,penilai_id,dinilai_id,arah,bintang,ulasan) VALUES (gen_random_uuid(),$1,$2,$3,$4,$5,$6) RETURNING id,iklan_id,penilai_id,dinilai_id,arah,bintang,ulasan,created_at,updated_at";
const SQL_EXISTS_FOR_PAIR: &str = "SELECT EXISTS(SELECT 1 FROM rating.rating WHERE iklan_id=$1 AND penilai_id=$2 AND dinilai_id=$3)";
/// Agregasi TIDAK memfilter modul/sumber (lihat doc `RatingAggregate`) — semua baris
/// `dinilai_id=$1` terhitung apa pun `iklan_id`-nya.
const SQL_AGGREGATE_FOR_USER: &str =
    "SELECT AVG(bintang)::float8 AS average, COUNT(*) AS count FROM rating.rating WHERE dinilai_id=$1";
