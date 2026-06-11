use std::time::Instant;

use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::entity::IklanPekerjaan;
use crate::domain::repository::IklanPekerjaanRepository;

pub struct PgIklanPekerjaanRepository {
    pool: PgPool,
}

impl PgIklanPekerjaanRepository {
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

impl IklanPekerjaanRepository for PgIklanPekerjaanRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<IklanPekerjaan>, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query!(
            "SELECT id,poster_id,judul,perusahaan,deskripsi,lokasi,gaji_min,gaji_max,tipe,is_active,created_at,updated_at FROM iklan_pekerjaan.iklan WHERE id=$1",
            id
        ).fetch_optional(&self.pool).await?;
        warn_slow!(t, "iklan_pekerjaan.find_by_id");
        Ok(r.map(|r| IklanPekerjaan {
            id: r.id,
            poster_id: r.poster_id,
            judul: r.judul,
            perusahaan: r.perusahaan,
            deskripsi: r.deskripsi,
            lokasi: r.lokasi,
            gaji_min: r.gaji_min,
            gaji_max: r.gaji_max,
            tipe: r.tipe,
            is_active: r.is_active,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }))
    }

    async fn list(&self, limit: i64, offset: i64) -> Result<Vec<IklanPekerjaan>, anyhow::Error> {
        let t = Instant::now();
        let rows = sqlx::query!(
            "SELECT id,poster_id,judul,perusahaan,deskripsi,lokasi,gaji_min,gaji_max,tipe,is_active,created_at,updated_at FROM iklan_pekerjaan.iklan WHERE is_active=true ORDER BY created_at DESC LIMIT $1 OFFSET $2",
            limit, offset
        ).fetch_all(&self.pool).await?;
        warn_slow!(t, "iklan_pekerjaan.list");
        Ok(rows
            .into_iter()
            .map(|r| IklanPekerjaan {
                id: r.id,
                poster_id: r.poster_id,
                judul: r.judul,
                perusahaan: r.perusahaan,
                deskripsi: r.deskripsi,
                lokasi: r.lokasi,
                gaji_min: r.gaji_min,
                gaji_max: r.gaji_max,
                tipe: r.tipe,
                is_active: r.is_active,
                created_at: r.created_at,
                updated_at: r.updated_at,
            })
            .collect())
    }

    async fn create(
        &self,
        poster_id: Uuid,
        judul: &str,
        perusahaan: &str,
        deskripsi: &str,
        tipe: &str,
    ) -> Result<IklanPekerjaan, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query!(
            "INSERT INTO iklan_pekerjaan.iklan (id,poster_id,judul,perusahaan,deskripsi,tipe) VALUES (gen_random_uuid(),$1,$2,$3,$4,$5) RETURNING id,poster_id,judul,perusahaan,deskripsi,lokasi,gaji_min,gaji_max,tipe,is_active,created_at,updated_at",
            poster_id, judul, perusahaan, deskripsi, tipe
        ).fetch_one(&self.pool).await?;
        warn_slow!(t, "iklan_pekerjaan.create");
        Ok(IklanPekerjaan {
            id: r.id,
            poster_id: r.poster_id,
            judul: r.judul,
            perusahaan: r.perusahaan,
            deskripsi: r.deskripsi,
            lokasi: r.lokasi,
            gaji_min: r.gaji_min,
            gaji_max: r.gaji_max,
            tipe: r.tipe,
            is_active: r.is_active,
            created_at: r.created_at,
            updated_at: r.updated_at,
        })
    }

    async fn delete(&self, id: Uuid, poster_id: Uuid) -> Result<bool, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query!(
            "DELETE FROM iklan_pekerjaan.iklan WHERE id=$1 AND poster_id=$2",
            id,
            poster_id
        )
        .execute(&self.pool)
        .await?;
        warn_slow!(t, "iklan_pekerjaan.delete");
        Ok(r.rows_affected() > 0)
    }

    async fn exists(&self, id: Uuid) -> Result<bool, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM iklan_pekerjaan.iklan WHERE id=$1)",
            id
        )
        .fetch_one(&self.pool)
        .await?
        .unwrap_or(false);
        warn_slow!(t, "iklan_pekerjaan.exists");
        Ok(r)
    }
}
