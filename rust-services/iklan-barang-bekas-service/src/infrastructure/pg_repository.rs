use std::time::Instant;

use crate::domain::entity::IklanBarangBekas;
use crate::domain::repository::IklanBarangBekasRepository;
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgIklanBarangBekasRepository {
    pool: PgPool,
}

impl PgIklanBarangBekasRepository {
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

impl IklanBarangBekasRepository for PgIklanBarangBekasRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<IklanBarangBekas>, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query!("SELECT id,seller_id,judul,deskripsi,harga,kondisi,lokasi,foto_urls,is_sold,created_at,updated_at FROM iklan_barang_bekas.iklan WHERE id=$1", id).fetch_optional(&self.pool).await?;
        warn_slow!(t, "iklan_barang_bekas.find_by_id");
        Ok(r.map(|r| IklanBarangBekas {
            id: r.id,
            seller_id: r.seller_id,
            judul: r.judul,
            deskripsi: r.deskripsi,
            harga: r.harga,
            kondisi: r.kondisi,
            lokasi: r.lokasi,
            foto_urls: r.foto_urls,
            is_sold: r.is_sold,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }))
    }

    async fn list(&self, limit: i64, offset: i64) -> Result<Vec<IklanBarangBekas>, anyhow::Error> {
        let t = Instant::now();
        let rows = sqlx::query!("SELECT id,seller_id,judul,deskripsi,harga,kondisi,lokasi,foto_urls,is_sold,created_at,updated_at FROM iklan_barang_bekas.iklan WHERE is_sold=false ORDER BY created_at DESC LIMIT $1 OFFSET $2", limit, offset).fetch_all(&self.pool).await?;
        warn_slow!(t, "iklan_barang_bekas.list");
        Ok(rows
            .into_iter()
            .map(|r| IklanBarangBekas {
                id: r.id,
                seller_id: r.seller_id,
                judul: r.judul,
                deskripsi: r.deskripsi,
                harga: r.harga,
                kondisi: r.kondisi,
                lokasi: r.lokasi,
                foto_urls: r.foto_urls,
                is_sold: r.is_sold,
                created_at: r.created_at,
                updated_at: r.updated_at,
            })
            .collect())
    }

    async fn create(
        &self,
        seller_id: Uuid,
        judul: &str,
        deskripsi: &str,
        harga: i64,
        kondisi: &str,
    ) -> Result<IklanBarangBekas, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query!("INSERT INTO iklan_barang_bekas.iklan (id,seller_id,judul,deskripsi,harga,kondisi) VALUES (gen_random_uuid(),$1,$2,$3,$4,$5) RETURNING id,seller_id,judul,deskripsi,harga,kondisi,lokasi,foto_urls,is_sold,created_at,updated_at", seller_id,judul,deskripsi,harga,kondisi).fetch_one(&self.pool).await?;
        warn_slow!(t, "iklan_barang_bekas.create");
        Ok(IklanBarangBekas {
            id: r.id,
            seller_id: r.seller_id,
            judul: r.judul,
            deskripsi: r.deskripsi,
            harga: r.harga,
            kondisi: r.kondisi,
            lokasi: r.lokasi,
            foto_urls: r.foto_urls,
            is_sold: r.is_sold,
            created_at: r.created_at,
            updated_at: r.updated_at,
        })
    }

    async fn mark_sold(&self, id: Uuid, seller_id: Uuid) -> Result<bool, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query!("UPDATE iklan_barang_bekas.iklan SET is_sold=true,updated_at=now() WHERE id=$1 AND seller_id=$2", id, seller_id).execute(&self.pool).await?;
        warn_slow!(t, "iklan_barang_bekas.mark_sold");
        Ok(r.rows_affected() > 0)
    }

    async fn delete(&self, id: Uuid, seller_id: Uuid) -> Result<bool, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query!(
            "DELETE FROM iklan_barang_bekas.iklan WHERE id=$1 AND seller_id=$2",
            id,
            seller_id
        )
        .execute(&self.pool)
        .await?;
        warn_slow!(t, "iklan_barang_bekas.delete");
        Ok(r.rows_affected() > 0)
    }

    async fn exists(&self, id: Uuid) -> Result<bool, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM iklan_barang_bekas.iklan WHERE id=$1)",
            id
        )
        .fetch_one(&self.pool)
        .await?
        .unwrap_or(false);
        warn_slow!(t, "iklan_barang_bekas.exists");
        Ok(r)
    }
}
