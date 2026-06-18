use std::time::Instant;

use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::domain::entity::{IklanPekerja, IklanSuspension, ModerationStatus};
use crate::domain::repository::{
    AdminListParams, AdminListResult, CreatePekerjaParams, IklanPekerjaRepository,
};

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

fn row_to_entity(r: &sqlx::postgres::PgRow) -> IklanPekerja {
    IklanPekerja {
        id: r.get("id"),
        poster_id: r.get("poster_id"),
        nama: r.get("nama"),
        keahlian: r.get("keahlian"),
        deskripsi: r.get("deskripsi"),
        lokasi: r.get("lokasi"),
        region_id: r.get("region_id"),
        tarif_min: r.get("tarif_min"),
        tarif_max: r.get("tarif_max"),
        foto_urls: r
            .get::<Option<Vec<String>>, _>("foto_urls")
            .unwrap_or_default(),
        is_active: r.get("is_active"),
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

impl IklanPekerjaRepository for PgIklanPekerjaRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<IklanPekerja>, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query("SELECT id,poster_id,nama,keahlian,deskripsi,lokasi,region_id,tarif_min,tarif_max,foto_urls,is_active,moderation_status,deleted_at,created_at,updated_at FROM iklan_pekerja.iklan WHERE id=$1")
            .bind(id).fetch_optional(&self.pool).await?;
        warn_slow!(t, "iklan_pekerja.find_by_id");
        Ok(r.as_ref().map(row_to_entity))
    }

    async fn list(&self, limit: i64, offset: i64) -> Result<Vec<IklanPekerja>, anyhow::Error> {
        let t = Instant::now();
        let rows = sqlx::query("SELECT id,poster_id,nama,keahlian,deskripsi,lokasi,region_id,tarif_min,tarif_max,foto_urls,is_active,moderation_status,deleted_at,created_at,updated_at FROM iklan_pekerja.iklan WHERE is_active=true AND moderation_status='active' AND deleted_at IS NULL ORDER BY created_at DESC LIMIT $1 OFFSET $2")
            .bind(limit).bind(offset).fetch_all(&self.pool).await?;
        warn_slow!(t, "iklan_pekerja.list");
        Ok(rows.iter().map(row_to_entity).collect())
    }

    async fn create(&self, params: CreatePekerjaParams<'_>) -> Result<IklanPekerja, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query(
            "INSERT INTO iklan_pekerja.iklan (id,poster_id,nama,keahlian,deskripsi,lokasi,region_id,tarif_min,tarif_max) VALUES (gen_random_uuid(),$1,$2,$3,$4,$5,$6,$7,$8) RETURNING id,poster_id,nama,keahlian,deskripsi,lokasi,region_id,tarif_min,tarif_max,foto_urls,is_active,moderation_status,deleted_at,created_at,updated_at",
        )
        .bind(params.poster_id).bind(params.nama).bind(params.keahlian).bind(params.deskripsi)
        .bind(params.lokasi).bind(params.region_id).bind(params.tarif_min).bind(params.tarif_max)
        .fetch_one(&self.pool).await?;
        warn_slow!(t, "iklan_pekerja.create");
        Ok(row_to_entity(&r))
    }

    async fn delete(&self, id: Uuid, poster_id: Uuid) -> Result<bool, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query("DELETE FROM iklan_pekerja.iklan WHERE id=$1 AND poster_id=$2")
            .bind(id)
            .bind(poster_id)
            .execute(&self.pool)
            .await?;
        warn_slow!(t, "iklan_pekerja.delete");
        Ok(r.rows_affected() > 0)
    }

    async fn exists(&self, id: Uuid) -> Result<bool, anyhow::Error> {
        let t = Instant::now();
        let r: Option<bool> =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM iklan_pekerja.iklan WHERE id=$1)")
                .bind(id)
                .fetch_one(&self.pool)
                .await?;
        warn_slow!(t, "iklan_pekerja.exists");
        Ok(r.unwrap_or(false))
    }

    async fn admin_list(&self, params: AdminListParams) -> Result<AdminListResult, anyhow::Error> {
        let t = Instant::now();
        let status = params.moderation_status.as_deref().unwrap_or("active");
        let q_pattern = params.q.as_deref().map(|s| format!("%{}%", s.trim()));
        let asc = matches!(params.sort_dir.as_deref(), Some("asc"));

        // Single query with COUNT(*) OVER() — eliminates extra round-trip.
        let rows = match (&q_pattern, asc) {
            (Some(q), true) => sqlx::query("SELECT id,poster_id,nama,keahlian,deskripsi,lokasi,region_id,tarif_min,tarif_max,foto_urls,is_active,moderation_status,deleted_at,created_at,updated_at,COUNT(*) OVER() AS total_rows FROM iklan_pekerja.iklan WHERE moderation_status=$1 AND deleted_at IS NULL AND (nama ILIKE $2 OR poster_id::text ILIKE $2 OR array_to_string(keahlian, ',') ILIKE $2) ORDER BY created_at ASC LIMIT $3 OFFSET $4")
                .bind(status).bind(q).bind(params.limit).bind(params.offset).fetch_all(&self.pool).await?,
            (Some(q), false) => sqlx::query("SELECT id,poster_id,nama,keahlian,deskripsi,lokasi,region_id,tarif_min,tarif_max,foto_urls,is_active,moderation_status,deleted_at,created_at,updated_at,COUNT(*) OVER() AS total_rows FROM iklan_pekerja.iklan WHERE moderation_status=$1 AND deleted_at IS NULL AND (nama ILIKE $2 OR poster_id::text ILIKE $2 OR array_to_string(keahlian, ',') ILIKE $2) ORDER BY created_at DESC LIMIT $3 OFFSET $4")
                .bind(status).bind(q).bind(params.limit).bind(params.offset).fetch_all(&self.pool).await?,
            (None, true) => sqlx::query("SELECT id,poster_id,nama,keahlian,deskripsi,lokasi,region_id,tarif_min,tarif_max,foto_urls,is_active,moderation_status,deleted_at,created_at,updated_at,COUNT(*) OVER() AS total_rows FROM iklan_pekerja.iklan WHERE moderation_status=$1 AND deleted_at IS NULL ORDER BY created_at ASC LIMIT $2 OFFSET $3")
                .bind(status).bind(params.limit).bind(params.offset).fetch_all(&self.pool).await?,
            (None, false) => sqlx::query("SELECT id,poster_id,nama,keahlian,deskripsi,lokasi,region_id,tarif_min,tarif_max,foto_urls,is_active,moderation_status,deleted_at,created_at,updated_at,COUNT(*) OVER() AS total_rows FROM iklan_pekerja.iklan WHERE moderation_status=$1 AND deleted_at IS NULL ORDER BY created_at DESC LIMIT $2 OFFSET $3")
                .bind(status).bind(params.limit).bind(params.offset).fetch_all(&self.pool).await?,
        };

        let total: i64 = rows
            .first()
            .map_or(0, |r| r.try_get::<i64, _>("total_rows").unwrap_or(0));

        warn_slow!(t, "iklan_pekerja.admin_list");
        Ok(AdminListResult {
            items: rows.iter().map(row_to_entity).collect(),
            total,
        })
    }

    async fn admin_list_all(
        &self,
        params: AdminListParams,
    ) -> Result<Vec<IklanPekerja>, anyhow::Error> {
        let t = Instant::now();
        let status = params.moderation_status.as_deref().unwrap_or("active");
        let q_pattern = params.q.as_deref().map(|s| format!("%{}%", s.trim()));

        let rows = match &q_pattern {
            Some(q) => sqlx::query("SELECT id,poster_id,nama,keahlian,deskripsi,lokasi,region_id,tarif_min,tarif_max,foto_urls,is_active,moderation_status,deleted_at,created_at,updated_at FROM iklan_pekerja.iklan WHERE moderation_status=$1 AND deleted_at IS NULL AND (nama ILIKE $2 OR poster_id::text ILIKE $2 OR array_to_string(keahlian, ',') ILIKE $2) ORDER BY created_at DESC LIMIT $3")
                .bind(status).bind(q).bind(params.limit).fetch_all(&self.pool).await?,
            None => sqlx::query("SELECT id,poster_id,nama,keahlian,deskripsi,lokasi,region_id,tarif_min,tarif_max,foto_urls,is_active,moderation_status,deleted_at,created_at,updated_at FROM iklan_pekerja.iklan WHERE moderation_status=$1 AND deleted_at IS NULL ORDER BY created_at DESC LIMIT $2")
                .bind(status).bind(params.limit).fetch_all(&self.pool).await?,
        };
        warn_slow!(t, "iklan_pekerja.admin_list_all");
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
            let updated = sqlx::query("UPDATE iklan_pekerja.iklan SET moderation_status=$1, updated_at=now() WHERE id=$2 AND deleted_at IS NULL RETURNING id")
                .bind(new_status).bind(iklan_id).fetch_optional(&self.pool).await?;
            warn_slow!(t, "iklan_pekerja.suspend_update");

            if updated.is_some() {
                let s = sqlx::query("INSERT INTO iklan_pekerja.iklan_suspension (id,iklan_id,is_permanent,reason,evidence_object_key,expires_at,created_by,created_at) VALUES (gen_random_uuid(),$1,$2,$3,$4,$5,$6,now()) RETURNING id,iklan_id,is_permanent,reason,evidence_object_key,expires_at,created_by,created_at")
                    .bind(iklan_id).bind(is_permanent).bind(reason).bind(evidence_object_key)
                    .bind(expires_at).bind(created_by).fetch_one(&self.pool).await?;
                suspensions.push(row_to_suspension(&s));
            }
        }
        Ok(suspensions)
    }

    async fn soft_delete(&self, id: Uuid) -> Result<bool, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query(
            "UPDATE iklan_pekerja.iklan SET deleted_at=now() WHERE id=$1 AND deleted_at IS NULL",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;
        warn_slow!(t, "iklan_pekerja.soft_delete");
        Ok(r.rows_affected() > 0)
    }

    async fn expire_temporary_suspensions(&self) -> Result<u64, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query(
            "UPDATE iklan_pekerja.iklan SET moderation_status='active', updated_at=now() WHERE moderation_status='suspended_temp' AND id IN (SELECT iklan_id FROM iklan_pekerja.iklan_suspension WHERE is_permanent=false AND expires_at IS NOT NULL AND expires_at <= now()) AND deleted_at IS NULL",
        )
        .execute(&self.pool)
        .await?;
        warn_slow!(t, "iklan_pekerja.expire_temporary_suspensions");
        Ok(r.rows_affected())
    }

    async fn is_poster_in_cooldown(&self, poster_id: Uuid) -> Result<bool, anyhow::Error> {
        let t = Instant::now();
        let r: Option<bool> = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM iklan_pekerja.iklan_suspension WHERE is_permanent=true AND created_at > now() - INTERVAL '3 days' AND iklan_id IN (SELECT id FROM iklan_pekerja.iklan WHERE poster_id=$1))",
        )
        .bind(poster_id)
        .fetch_one(&self.pool)
        .await?;
        warn_slow!(t, "iklan_pekerja.is_poster_in_cooldown");
        Ok(r.unwrap_or(false))
    }
}
