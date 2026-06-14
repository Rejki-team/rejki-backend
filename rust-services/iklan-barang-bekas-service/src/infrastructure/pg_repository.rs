use std::time::Instant;

use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::domain::entity::{IklanBarangBekas, IklanSuspension, ModerationStatus};
use crate::domain::repository::{
    AdminListParams, AdminListResult, CreateBarangBekasParams, IklanBarangBekasRepository,
};

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

fn row_to_entity(r: &sqlx::postgres::PgRow) -> IklanBarangBekas {
    IklanBarangBekas {
        id: r.get("id"),
        seller_id: r.get("seller_id"),
        judul: r.get("judul"),
        deskripsi: r.get("deskripsi"),
        harga: r.get("harga"),
        kondisi: r.get("kondisi"),
        lokasi: r.get("lokasi"),
        foto_urls: r.get("foto_urls"),
        is_sold: r.get("is_sold"),
        moderation_status: ModerationStatus::parse(&r.get::<String, _>("moderation_status"))
            .unwrap_or_default(),
        deleted_at: r.get("deleted_at"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    }
}

fn row_to_suspension(r: &sqlx::postgres::PgRow) -> IklanSuspension {
    IklanSuspension {
        id: r.get("id"),
        iklan_id: r.get("iklan_id"),
        is_permanent: r.get("is_permanent"),
        reason: r.get("reason"),
        evidence_object_key: r.get("evidence_object_key"),
        expires_at: r.get("expires_at"),
        created_by: r.get("created_by"),
        created_at: r.get("created_at"),
    }
}

impl IklanBarangBekasRepository for PgIklanBarangBekasRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<IklanBarangBekas>, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query("SELECT id,seller_id,judul,deskripsi,harga,kondisi,lokasi,foto_urls,is_sold,moderation_status,deleted_at,created_at,updated_at FROM iklan_barang_bekas.iklan WHERE id=$1")
            .bind(id).fetch_optional(&self.pool).await?;
        warn_slow!(t, "iklan_barang_bekas.find_by_id");
        Ok(r.as_ref().map(row_to_entity))
    }

    async fn list(&self, limit: i64, offset: i64) -> Result<Vec<IklanBarangBekas>, anyhow::Error> {
        let t = Instant::now();
        let rows = sqlx::query("SELECT id,seller_id,judul,deskripsi,harga,kondisi,lokasi,foto_urls,is_sold,moderation_status,deleted_at,created_at,updated_at FROM iklan_barang_bekas.iklan WHERE is_sold=false AND moderation_status='active' AND deleted_at IS NULL ORDER BY created_at DESC LIMIT $1 OFFSET $2")
            .bind(limit).bind(offset).fetch_all(&self.pool).await?;
        warn_slow!(t, "iklan_barang_bekas.list");
        Ok(rows.iter().map(row_to_entity).collect())
    }

    async fn create(
        &self,
        params: CreateBarangBekasParams<'_>,
    ) -> Result<IklanBarangBekas, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query("INSERT INTO iklan_barang_bekas.iklan (id,seller_id,judul,deskripsi,harga,kondisi,lokasi,foto_urls) VALUES (gen_random_uuid(),$1,$2,$3,$4,$5,$6,$7) RETURNING id,seller_id,judul,deskripsi,harga,kondisi,lokasi,foto_urls,is_sold,moderation_status,deleted_at,created_at,updated_at")
            .bind(params.seller_id).bind(params.judul).bind(params.deskripsi).bind(params.harga).bind(params.kondisi)
            .bind(params.lokasi).bind(params.foto_urls).fetch_one(&self.pool).await?;
        warn_slow!(t, "iklan_barang_bekas.create");
        Ok(row_to_entity(&r))
    }

    async fn mark_sold(&self, id: Uuid, seller_id: Uuid) -> Result<bool, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query("UPDATE iklan_barang_bekas.iklan SET is_sold=true,updated_at=now() WHERE id=$1 AND seller_id=$2")
            .bind(id).bind(seller_id).execute(&self.pool).await?;
        warn_slow!(t, "iklan_barang_bekas.mark_sold");
        Ok(r.rows_affected() > 0)
    }

    async fn delete(&self, id: Uuid, seller_id: Uuid) -> Result<bool, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query("DELETE FROM iklan_barang_bekas.iklan WHERE id=$1 AND seller_id=$2")
            .bind(id)
            .bind(seller_id)
            .execute(&self.pool)
            .await?;
        warn_slow!(t, "iklan_barang_bekas.delete");
        Ok(r.rows_affected() > 0)
    }

    async fn exists(&self, id: Uuid) -> Result<bool, anyhow::Error> {
        let t = Instant::now();
        let r: Option<bool> =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM iklan_barang_bekas.iklan WHERE id=$1)")
                .bind(id)
                .fetch_one(&self.pool)
                .await?;
        warn_slow!(t, "iklan_barang_bekas.exists");
        Ok(r.unwrap_or(false))
    }

    async fn admin_list(&self, params: AdminListParams) -> Result<AdminListResult, anyhow::Error> {
        let t = Instant::now();
        let status = params.moderation_status.as_deref().unwrap_or("active");
        let q_pattern = params.q.as_deref().map(|s| format!("%{}%", s.trim()));
        let asc = matches!(params.sort_dir.as_deref(), Some("asc"));

        // Single query with COUNT(*) OVER() — eliminates extra round-trip.
        let rows = match (&q_pattern, asc) {
            (Some(q), true) => sqlx::query("SELECT id,seller_id,judul,deskripsi,harga,kondisi,lokasi,foto_urls,is_sold,moderation_status,deleted_at,created_at,updated_at,COUNT(*) OVER() AS total_rows FROM iklan_barang_bekas.iklan WHERE moderation_status=$1 AND deleted_at IS NULL AND (judul ILIKE $2 OR seller_id::text ILIKE $2) ORDER BY created_at ASC LIMIT $3 OFFSET $4")
                .bind(status).bind(q).bind(params.limit).bind(params.offset).fetch_all(&self.pool).await?,
            (Some(q), false) => sqlx::query("SELECT id,seller_id,judul,deskripsi,harga,kondisi,lokasi,foto_urls,is_sold,moderation_status,deleted_at,created_at,updated_at,COUNT(*) OVER() AS total_rows FROM iklan_barang_bekas.iklan WHERE moderation_status=$1 AND deleted_at IS NULL AND (judul ILIKE $2 OR seller_id::text ILIKE $2) ORDER BY created_at DESC LIMIT $3 OFFSET $4")
                .bind(status).bind(q).bind(params.limit).bind(params.offset).fetch_all(&self.pool).await?,
            (None, true) => sqlx::query("SELECT id,seller_id,judul,deskripsi,harga,kondisi,lokasi,foto_urls,is_sold,moderation_status,deleted_at,created_at,updated_at,COUNT(*) OVER() AS total_rows FROM iklan_barang_bekas.iklan WHERE moderation_status=$1 AND deleted_at IS NULL ORDER BY created_at ASC LIMIT $2 OFFSET $3")
                .bind(status).bind(params.limit).bind(params.offset).fetch_all(&self.pool).await?,
            (None, false) => sqlx::query("SELECT id,seller_id,judul,deskripsi,harga,kondisi,lokasi,foto_urls,is_sold,moderation_status,deleted_at,created_at,updated_at,COUNT(*) OVER() AS total_rows FROM iklan_barang_bekas.iklan WHERE moderation_status=$1 AND deleted_at IS NULL ORDER BY created_at DESC LIMIT $2 OFFSET $3")
                .bind(status).bind(params.limit).bind(params.offset).fetch_all(&self.pool).await?,
        };

        let total: i64 = rows
            .first()
            .map_or(0, |r| r.try_get::<i64, _>("total_rows").unwrap_or(0));
        warn_slow!(t, "iklan_barang_bekas.admin_list");
        Ok(AdminListResult {
            items: rows.iter().map(row_to_entity).collect(),
            total,
        })
    }

    async fn admin_list_all(
        &self,
        params: AdminListParams,
    ) -> Result<Vec<IklanBarangBekas>, anyhow::Error> {
        let t = Instant::now();
        let status = params.moderation_status.as_deref().unwrap_or("active");
        let q_pattern = params.q.as_deref().map(|s| format!("%{}%", s.trim()));

        let rows = match &q_pattern {
            Some(q) => sqlx::query("SELECT id,seller_id,judul,deskripsi,harga,kondisi,lokasi,foto_urls,is_sold,moderation_status,deleted_at,created_at,updated_at FROM iklan_barang_bekas.iklan WHERE moderation_status=$1 AND deleted_at IS NULL AND (judul ILIKE $2 OR seller_id::text ILIKE $2) ORDER BY created_at DESC LIMIT $3")
                .bind(status).bind(q).bind(params.limit).fetch_all(&self.pool).await?,
            None => sqlx::query("SELECT id,seller_id,judul,deskripsi,harga,kondisi,lokasi,foto_urls,is_sold,moderation_status,deleted_at,created_at,updated_at FROM iklan_barang_bekas.iklan WHERE moderation_status=$1 AND deleted_at IS NULL ORDER BY created_at DESC LIMIT $2")
                .bind(status).bind(params.limit).fetch_all(&self.pool).await?,
        };
        warn_slow!(t, "iklan_barang_bekas.admin_list_all");
        Ok(rows.iter().map(row_to_entity).collect())
    }

    async fn suspend(
        &self,
        iklan_ids: &[Uuid],
        is_permanent: bool,
        reason: &str,
        evidence_object_key: Option<&str>,
        expires_at: Option<DateTime<Utc>>,
        created_by: Uuid,
    ) -> Result<Vec<IklanSuspension>, anyhow::Error> {
        let new_status = if is_permanent {
            "suspended_permanent"
        } else {
            "suspended_temp"
        };
        let mut suspensions = Vec::new();

        for &iklan_id in iklan_ids {
            let t = Instant::now();
            let updated = sqlx::query("UPDATE iklan_barang_bekas.iklan SET moderation_status=$1, updated_at=now() WHERE id=$2 AND deleted_at IS NULL RETURNING id")
                .bind(new_status).bind(iklan_id).fetch_optional(&self.pool).await?;
            warn_slow!(t, "iklan_barang_bekas.suspend_update");

            if updated.is_some() {
                let s = sqlx::query("INSERT INTO iklan_barang_bekas.iklan_suspension (id,iklan_id,is_permanent,reason,evidence_object_key,expires_at,created_by,created_at) VALUES (gen_random_uuid(),$1,$2,$3,$4,$5,$6,now()) RETURNING id,iklan_id,is_permanent,reason,evidence_object_key,expires_at,created_by,created_at")
                    .bind(iklan_id).bind(is_permanent).bind(reason).bind(evidence_object_key)
                    .bind(expires_at).bind(created_by).fetch_one(&self.pool).await?;
                suspensions.push(row_to_suspension(&s));
            }
        }
        Ok(suspensions)
    }

    async fn soft_delete(&self, id: Uuid) -> Result<bool, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query("UPDATE iklan_barang_bekas.iklan SET deleted_at=now() WHERE id=$1 AND deleted_at IS NULL")
            .bind(id).execute(&self.pool).await?;
        warn_slow!(t, "iklan_barang_bekas.soft_delete");
        Ok(r.rows_affected() > 0)
    }

    async fn expire_temporary_suspensions(&self) -> Result<u64, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query(
            "UPDATE iklan_barang_bekas.iklan SET moderation_status='active', updated_at=now() WHERE moderation_status='suspended_temp' AND id IN (SELECT iklan_id FROM iklan_barang_bekas.iklan_suspension WHERE is_permanent=false AND expires_at IS NOT NULL AND expires_at <= now()) AND deleted_at IS NULL",
        )
        .execute(&self.pool)
        .await?;
        warn_slow!(t, "iklan_barang_bekas.expire_temporary_suspensions");
        Ok(r.rows_affected())
    }

    async fn is_poster_in_cooldown(&self, poster_id: Uuid) -> Result<bool, anyhow::Error> {
        let t = Instant::now();
        let r: Option<bool> = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM iklan_barang_bekas.iklan_suspension WHERE is_permanent=true AND created_at > now() - INTERVAL '3 days' AND iklan_id IN (SELECT id FROM iklan_barang_bekas.iklan WHERE seller_id=$1))",
        )
        .bind(poster_id)
        .fetch_one(&self.pool)
        .await?;
        warn_slow!(t, "iklan_barang_bekas.is_poster_in_cooldown");
        Ok(r.unwrap_or(false))
    }
}
