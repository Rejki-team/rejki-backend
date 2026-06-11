use std::time::Instant;

use crate::domain::entity::IklanPekerja;
use crate::domain::repository::IklanPekerjaRepository;
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgIklanPekerjaRepository {
    pool: PgPool,
}

impl PgIklanPekerjaRepository {
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

impl IklanPekerjaRepository for PgIklanPekerjaRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<IklanPekerja>, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query!("SELECT id,poster_id,nama,keahlian,deskripsi,lokasi,tarif_min,tarif_max,is_active,created_at,updated_at FROM iklan_pekerja.iklan WHERE id=$1", id).fetch_optional(&self.pool).await?;
        warn_slow!(t, "iklan_pekerja.find_by_id");
        Ok(r.map(|r| IklanPekerja {
            id: r.id,
            poster_id: r.poster_id,
            nama: r.nama,
            keahlian: r.keahlian,
            deskripsi: r.deskripsi,
            lokasi: r.lokasi,
            tarif_min: r.tarif_min,
            tarif_max: r.tarif_max,
            is_active: r.is_active,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }))
    }

    async fn list(&self, limit: i64, offset: i64) -> Result<Vec<IklanPekerja>, anyhow::Error> {
        let t = Instant::now();
        let rows = sqlx::query!("SELECT id,poster_id,nama,keahlian,deskripsi,lokasi,tarif_min,tarif_max,is_active,created_at,updated_at FROM iklan_pekerja.iklan WHERE is_active=true ORDER BY created_at DESC LIMIT $1 OFFSET $2", limit, offset).fetch_all(&self.pool).await?;
        warn_slow!(t, "iklan_pekerja.list");
        Ok(rows
            .into_iter()
            .map(|r| IklanPekerja {
                id: r.id,
                poster_id: r.poster_id,
                nama: r.nama,
                keahlian: r.keahlian,
                deskripsi: r.deskripsi,
                lokasi: r.lokasi,
                tarif_min: r.tarif_min,
                tarif_max: r.tarif_max,
                is_active: r.is_active,
                created_at: r.created_at,
                updated_at: r.updated_at,
            })
            .collect())
    }

    async fn create(
        &self,
        poster_id: Uuid,
        nama: &str,
        keahlian: &[String],
        deskripsi: &str,
    ) -> Result<IklanPekerja, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query!("INSERT INTO iklan_pekerja.iklan (id,poster_id,nama,keahlian,deskripsi) VALUES (gen_random_uuid(),$1,$2,$3,$4) RETURNING id,poster_id,nama,keahlian,deskripsi,lokasi,tarif_min,tarif_max,is_active,created_at,updated_at", poster_id, nama, keahlian, deskripsi).fetch_one(&self.pool).await?;
        warn_slow!(t, "iklan_pekerja.create");
        Ok(IklanPekerja {
            id: r.id,
            poster_id: r.poster_id,
            nama: r.nama,
            keahlian: r.keahlian,
            deskripsi: r.deskripsi,
            lokasi: r.lokasi,
            tarif_min: r.tarif_min,
            tarif_max: r.tarif_max,
            is_active: r.is_active,
            created_at: r.created_at,
            updated_at: r.updated_at,
        })
    }

    async fn delete(&self, id: Uuid, poster_id: Uuid) -> Result<bool, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query!(
            "DELETE FROM iklan_pekerja.iklan WHERE id=$1 AND poster_id=$2",
            id,
            poster_id
        )
        .execute(&self.pool)
        .await?;
        warn_slow!(t, "iklan_pekerja.delete");
        Ok(r.rows_affected() > 0)
    }

    async fn exists(&self, id: Uuid) -> Result<bool, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM iklan_pekerja.iklan WHERE id=$1)",
            id
        )
        .fetch_one(&self.pool)
        .await?
        .unwrap_or(false);
        warn_slow!(t, "iklan_pekerja.exists");
        Ok(r)
    }
}
