use std::time::Instant;

use crate::domain::entity::IklanPelatihan;
use crate::domain::repository::IklanPelatihanRepository;
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgIklanPelatihanRepository {
    pool: PgPool,
}

impl PgIklanPelatihanRepository {
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

impl IklanPelatihanRepository for PgIklanPelatihanRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<IklanPelatihan>, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query!("SELECT id,poster_id,judul,penyelenggara,deskripsi,lokasi,harga,tanggal_mulai,tanggal_selesai,is_active,created_at,updated_at FROM iklan_pelatihan.iklan WHERE id=$1", id).fetch_optional(&self.pool).await?;
        warn_slow!(t, "iklan_pelatihan.find_by_id");
        Ok(r.map(|r| IklanPelatihan {
            id: r.id,
            poster_id: r.poster_id,
            judul: r.judul,
            penyelenggara: r.penyelenggara,
            deskripsi: r.deskripsi,
            lokasi: r.lokasi,
            harga: r.harga,
            tanggal_mulai: r.tanggal_mulai,
            tanggal_selesai: r.tanggal_selesai,
            is_active: r.is_active,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }))
    }

    async fn list(&self, limit: i64, offset: i64) -> Result<Vec<IklanPelatihan>, anyhow::Error> {
        let t = Instant::now();
        let rows = sqlx::query!("SELECT id,poster_id,judul,penyelenggara,deskripsi,lokasi,harga,tanggal_mulai,tanggal_selesai,is_active,created_at,updated_at FROM iklan_pelatihan.iklan WHERE is_active=true ORDER BY created_at DESC LIMIT $1 OFFSET $2", limit, offset).fetch_all(&self.pool).await?;
        warn_slow!(t, "iklan_pelatihan.list");
        Ok(rows
            .into_iter()
            .map(|r| IklanPelatihan {
                id: r.id,
                poster_id: r.poster_id,
                judul: r.judul,
                penyelenggara: r.penyelenggara,
                deskripsi: r.deskripsi,
                lokasi: r.lokasi,
                harga: r.harga,
                tanggal_mulai: r.tanggal_mulai,
                tanggal_selesai: r.tanggal_selesai,
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
        penyelenggara: &str,
        deskripsi: &str,
    ) -> Result<IklanPelatihan, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query!("INSERT INTO iklan_pelatihan.iklan (id,poster_id,judul,penyelenggara,deskripsi) VALUES (gen_random_uuid(),$1,$2,$3,$4) RETURNING id,poster_id,judul,penyelenggara,deskripsi,lokasi,harga,tanggal_mulai,tanggal_selesai,is_active,created_at,updated_at", poster_id,judul,penyelenggara,deskripsi).fetch_one(&self.pool).await?;
        warn_slow!(t, "iklan_pelatihan.create");
        Ok(IklanPelatihan {
            id: r.id,
            poster_id: r.poster_id,
            judul: r.judul,
            penyelenggara: r.penyelenggara,
            deskripsi: r.deskripsi,
            lokasi: r.lokasi,
            harga: r.harga,
            tanggal_mulai: r.tanggal_mulai,
            tanggal_selesai: r.tanggal_selesai,
            is_active: r.is_active,
            created_at: r.created_at,
            updated_at: r.updated_at,
        })
    }

    async fn delete(&self, id: Uuid, poster_id: Uuid) -> Result<bool, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query!(
            "DELETE FROM iklan_pelatihan.iklan WHERE id=$1 AND poster_id=$2",
            id,
            poster_id
        )
        .execute(&self.pool)
        .await?;
        warn_slow!(t, "iklan_pelatihan.delete");
        Ok(r.rows_affected() > 0)
    }

    async fn exists(&self, id: Uuid) -> Result<bool, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM iklan_pelatihan.iklan WHERE id=$1)",
            id
        )
        .fetch_one(&self.pool)
        .await?
        .unwrap_or(false);
        warn_slow!(t, "iklan_pelatihan.exists");
        Ok(r)
    }
}
