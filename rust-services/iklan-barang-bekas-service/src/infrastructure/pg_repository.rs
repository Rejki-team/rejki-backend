use std::time::Instant;

use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::domain::entity::{
    AvailabilityStatus, Bider, BiderStatus, IklanBarangBekas, IklanSuspension, ModerationStatus,
};
use crate::domain::repository::{
    AdminListParams, AdminListResult, CreateBarangBekasParams, IklanBarangBekasRepository,
    UpdateBarangBekasParams,
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
    let moderation_raw: String = r.get("moderation_status");
    let avail_raw: String = r.get("availability_status");
    IklanBarangBekas {
        id: r.get("id"),
        seller_id: r.get("seller_id"),
        judul: r.get("judul"),
        deskripsi: r.get("deskripsi"),
        jenis_barang: r.get("jenis_barang"),
        jumlah: r.get("jumlah"),
        lokasi_pengambilan: r.get("lokasi_pengambilan"),
        lokasi: r.get("lokasi"),
        region_id: r.get("region_id"),
        foto_urls: r.get("foto_urls"),
        availability_status: AvailabilityStatus::parse(&avail_raw).unwrap_or_default(),
        moderation_status: ModerationStatus::parse(&moderation_raw).unwrap_or_default(),
        deleted_at: r.get("deleted_at"),
        latitude: r.get("latitude"),
        longitude: r.get("longitude"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    }
}

fn row_to_bider(r: &sqlx::postgres::PgRow) -> Bider {
    let status_raw: String = r.get("status");
    Bider {
        id: r.get("id"),
        iklan_id: r.get("iklan_id"),
        peminat_id: r.get("peminat_id"),
        status: BiderStatus::parse(&status_raw).unwrap_or_default(),
        sudah_menghubungi: r.get("sudah_menghubungi"),
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
        let r = sqlx::query(SQL_FIND_BY_ID)
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        warn_slow!(t, "iklan_barang_bekas.find_by_id");
        Ok(r.as_ref().map(row_to_entity))
    }

    async fn list(
        &self,
        limit: i64,
        offset: i64,
        radius: Option<common_geo::RadiusQuery>,
    ) -> Result<Vec<IklanBarangBekas>, anyhow::Error> {
        let t = Instant::now();
        let rows = if let Some(r) = radius {
            let bb = common_geo::bounding_box(r.lat, r.lng, r.radius_km);
            sqlx::query(SQL_LIST_RADIUS)
                .bind(limit)
                .bind(offset)
                .bind(bb.min_lat)
                .bind(bb.max_lat)
                .bind(bb.min_lng)
                .bind(bb.max_lng)
                .bind(r.lat)
                .bind(r.lng)
                .bind(r.radius_km)
                .fetch_all(&self.pool)
                .await?
        } else {
            sqlx::query(SQL_LIST)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
        };
        warn_slow!(t, "iklan_barang_bekas.list");
        Ok(rows.iter().map(row_to_entity).collect())
    }

    async fn create(
        &self,
        params: CreateBarangBekasParams<'_>,
    ) -> Result<IklanBarangBekas, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query(SQL_CREATE)
            .bind(params.seller_id)
            .bind(params.judul)
            .bind(params.deskripsi)
            .bind(params.jenis_barang)
            .bind(params.jumlah)
            .bind(params.lokasi_pengambilan)
            .bind(params.lokasi)
            .bind(params.region_id)
            .bind(params.foto_urls)
            .bind(params.latitude)
            .bind(params.longitude)
            .fetch_one(&self.pool)
            .await?;
        warn_slow!(t, "iklan_barang_bekas.create");
        Ok(row_to_entity(&r))
    }

    async fn mark_taken(&self, id: Uuid, seller_id: Uuid) -> Result<bool, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query(SQL_MARK_TAKEN)
            .bind(id)
            .bind(seller_id)
            .execute(&self.pool)
            .await?;
        warn_slow!(t, "iklan_barang_bekas.mark_taken");
        Ok(r.rows_affected() > 0)
    }

    async fn delete(&self, id: Uuid, seller_id: Uuid) -> Result<bool, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query(SQL_DELETE)
            .bind(id)
            .bind(seller_id)
            .execute(&self.pool)
            .await?;
        warn_slow!(t, "iklan_barang_bekas.delete");
        Ok(r.rows_affected() > 0)
    }

    async fn update(
        &self,
        id: Uuid,
        seller_id: Uuid,
        params: UpdateBarangBekasParams,
    ) -> Result<Option<IklanBarangBekas>, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query(SQL_UPDATE)
            .bind(id)
            .bind(seller_id)
            .bind(&params.judul)
            .bind(&params.deskripsi)
            .bind(&params.jenis_barang)
            .bind(params.jumlah)
            .bind(&params.lokasi_pengambilan)
            .bind(&params.lokasi)
            .bind(&params.region_id)
            .bind(&params.foto_urls)
            .bind(params.is_active)
            .bind(params.latitude)
            .bind(params.longitude)
            .fetch_optional(&self.pool)
            .await?;
        warn_slow!(t, "iklan_barang_bekas.update");
        Ok(r.as_ref().map(row_to_entity))
    }

    async fn exists(&self, id: Uuid) -> Result<bool, anyhow::Error> {
        let t = Instant::now();
        let r: Option<bool> = sqlx::query_scalar(SQL_EXISTS)
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

        let rows = match (&q_pattern, asc) {
            (Some(q), true) => {
                sqlx::query(ADMIN_LIST_SEARCH_ASC)
                    .bind(status)
                    .bind(q)
                    .bind(params.limit)
                    .bind(params.offset)
                    .fetch_all(&self.pool)
                    .await?
            }
            (Some(q), false) => {
                sqlx::query(ADMIN_LIST_SEARCH_DESC)
                    .bind(status)
                    .bind(q)
                    .bind(params.limit)
                    .bind(params.offset)
                    .fetch_all(&self.pool)
                    .await?
            }
            (None, true) => {
                sqlx::query(ADMIN_LIST_ASC)
                    .bind(status)
                    .bind(params.limit)
                    .bind(params.offset)
                    .fetch_all(&self.pool)
                    .await?
            }
            (None, false) => {
                sqlx::query(ADMIN_LIST_DESC)
                    .bind(status)
                    .bind(params.limit)
                    .bind(params.offset)
                    .fetch_all(&self.pool)
                    .await?
            }
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
            Some(q) => {
                sqlx::query(ADMIN_EXPORT_SEARCH)
                    .bind(status)
                    .bind(q)
                    .bind(params.limit)
                    .fetch_all(&self.pool)
                    .await?
            }
            None => {
                sqlx::query(ADMIN_EXPORT)
                    .bind(status)
                    .bind(params.limit)
                    .fetch_all(&self.pool)
                    .await?
            }
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
            let updated = sqlx::query(SQL_SUSPEND_UPDATE)
                .bind(new_status)
                .bind(iklan_id)
                .fetch_optional(&self.pool)
                .await?;
            warn_slow!(t, "iklan_barang_bekas.suspend_update");

            if updated.is_some() {
                let s = sqlx::query(SQL_SUSPEND_INSERT)
                    .bind(iklan_id)
                    .bind(is_permanent)
                    .bind(reason)
                    .bind(evidence_object_key)
                    .bind(expires_at)
                    .bind(created_by)
                    .fetch_one(&self.pool)
                    .await?;
                suspensions.push(row_to_suspension(&s));
            }
        }
        Ok(suspensions)
    }

    async fn soft_delete(&self, id: Uuid) -> Result<bool, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query(SQL_SOFT_DELETE)
            .bind(id)
            .execute(&self.pool)
            .await?;
        warn_slow!(t, "iklan_barang_bekas.soft_delete");
        Ok(r.rows_affected() > 0)
    }

    async fn expire_temporary_suspensions(&self) -> Result<u64, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query(SQL_EXPIRE_TEMP).execute(&self.pool).await?;
        warn_slow!(t, "iklan_barang_bekas.expire_temporary_suspensions");
        Ok(r.rows_affected())
    }

    async fn is_poster_in_cooldown(&self, poster_id: Uuid) -> Result<bool, anyhow::Error> {
        let t = Instant::now();
        let r: Option<bool> = sqlx::query_scalar(SQL_COOLDOWN)
            .bind(poster_id)
            .fetch_one(&self.pool)
            .await?;
        warn_slow!(t, "iklan_barang_bekas.is_poster_in_cooldown");
        Ok(r.unwrap_or(false))
    }

    async fn list_by_seller(
        &self,
        seller_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<IklanBarangBekas>, anyhow::Error> {
        let t = Instant::now();
        let rows = sqlx::query(SQL_LIST_BY_SELLER)
            .bind(seller_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;
        warn_slow!(t, "iklan_barang_bekas.list_by_seller");
        Ok(rows.iter().map(row_to_entity).collect())
    }

    async fn find_by_ids(&self, ids: &[Uuid]) -> Result<Vec<IklanBarangBekas>, anyhow::Error> {
        let t = Instant::now();
        let rows = sqlx::query(SQL_FIND_BY_IDS)
            .bind(ids)
            .fetch_all(&self.pool)
            .await?;
        warn_slow!(t, "iklan_barang_bekas.find_by_ids");
        Ok(rows.iter().map(row_to_entity).collect())
    }

    // ── Bider (F-15, Kelompok 3 Phase 3) ────────────────────────────────────

    async fn find_bider_by_id(&self, id: Uuid) -> Result<Option<Bider>, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query(SQL_BIDER_FIND_BY_ID)
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        warn_slow!(t, "iklan_barang_bekas.find_bider_by_id");
        Ok(r.as_ref().map(row_to_bider))
    }

    async fn create_bider(&self, iklan_id: Uuid, peminat_id: Uuid) -> Result<Bider, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query(SQL_BIDER_CREATE)
            .bind(iklan_id)
            .bind(peminat_id)
            .fetch_one(&self.pool)
            .await?;
        warn_slow!(t, "iklan_barang_bekas.create_bider");
        Ok(row_to_bider(&r))
    }

    async fn list_bider_for_iklan(&self, iklan_id: Uuid) -> Result<Vec<Bider>, anyhow::Error> {
        let t = Instant::now();
        let rows = sqlx::query(SQL_BIDER_LIST_FOR_IKLAN)
            .bind(iklan_id)
            .fetch_all(&self.pool)
            .await?;
        warn_slow!(t, "iklan_barang_bekas.list_bider_for_iklan");
        Ok(rows.iter().map(row_to_bider).collect())
    }

    async fn has_pending_bider(
        &self,
        iklan_id: Uuid,
        peminat_id: Uuid,
    ) -> Result<bool, anyhow::Error> {
        let t = Instant::now();
        let r: Option<bool> = sqlx::query_scalar(SQL_BIDER_HAS_PENDING)
            .bind(iklan_id)
            .bind(peminat_id)
            .fetch_one(&self.pool)
            .await?;
        warn_slow!(t, "iklan_barang_bekas.has_pending_bider");
        Ok(r.unwrap_or(false))
    }

    async fn setujui_bider(
        &self,
        bider_id: Uuid,
        iklan_owner_id: Uuid,
        sudah_menghubungi: bool,
    ) -> Result<Option<Bider>, anyhow::Error> {
        let t = Instant::now();
        let mut tx = self.pool.begin().await?;

        let r = sqlx::query(SQL_BIDER_SETUJUI)
            .bind(sudah_menghubungi)
            .bind(bider_id)
            .bind(iklan_owner_id)
            .fetch_optional(&mut *tx)
            .await?;

        let bider = match r {
            Some(row) => row_to_bider(&row),
            None => {
                tx.rollback().await?;
                warn_slow!(t, "iklan_barang_bekas.setujui_bider");
                return Ok(None);
            }
        };

        sqlx::query(SQL_BIDER_SETUJUI_MARK_IKLAN_TAKEN)
            .bind(bider.iklan_id)
            .execute(&mut *tx)
            .await?;

        // Tandai bider lain (bila ada) untuk iklan ini sebagai tidak relevan (P3.4).
        sqlx::query(SQL_BIDER_WITHDRAW_OTHERS)
            .bind(bider.iklan_id)
            .bind(bider.id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        warn_slow!(t, "iklan_barang_bekas.setujui_bider");
        Ok(Some(bider))
    }

    async fn withdraw_bider(
        &self,
        bider_id: Uuid,
        iklan_owner_id: Uuid,
    ) -> Result<Option<Bider>, anyhow::Error> {
        let t = Instant::now();
        let mut tx = self.pool.begin().await?;

        // Kunci baris + baca status LAMA (perlu tahu apakah sebelumnya `disetujui` untuk
        // menentukan re-listing) sebelum di-UPDATE — ownership check DI QUERY (IDOR→404).
        let existing = sqlx::query(SQL_BIDER_LOCK_FOR_WITHDRAW)
            .bind(bider_id)
            .bind(iklan_owner_id)
            .fetch_optional(&mut *tx)
            .await?;

        let old = match existing {
            Some(row) => row_to_bider(&row),
            None => {
                tx.rollback().await?;
                warn_slow!(t, "iklan_barang_bekas.withdraw_bider");
                return Ok(None);
            }
        };

        let updated = sqlx::query(SQL_BIDER_WITHDRAW)
            .bind(bider_id)
            .fetch_one(&mut *tx)
            .await?;

        // Re-listing (P3.5): hanya bila bider yang di-withdraw SEBELUMNYA `disetujui`
        // (iklan sudah SudahDiambil) — kembalikan ke Tersedia. Bila sebelumnya `menunggu`,
        // iklan tidak pernah delisted, tidak ada state untuk dikembalikan.
        if old.status == BiderStatus::Disetujui {
            sqlx::query(SQL_BIDER_WITHDRAW_RELIST_IKLAN)
                .bind(old.iklan_id)
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;
        warn_slow!(t, "iklan_barang_bekas.withdraw_bider");
        Ok(Some(row_to_bider(&updated)))
    }

    async fn list_bider_for_peminat(&self, peminat_id: Uuid) -> Result<Vec<Bider>, anyhow::Error> {
        let t = Instant::now();
        let rows = sqlx::query(SQL_BIDER_LIST_FOR_PEMINAT)
            .bind(peminat_id)
            .fetch_all(&self.pool)
            .await?;
        warn_slow!(t, "iklan_barang_bekas.list_bider_for_peminat");
        Ok(rows.iter().map(row_to_bider).collect())
    }
}

// ── SQL literal statis — sqlx 0.9 anti-dynamic-string; literal = anti SQL-injection ──
// Kolom baru (gratis/donasi): jenis_barang, jumlah, lokasi_pengambilan, availability_status.
// Kolom dihapus: harga, kondisi, is_sold.

const SQL_FIND_BY_ID: &str = "SELECT id,seller_id,judul,deskripsi,jenis_barang,jumlah,lokasi_pengambilan,lokasi,region_id,foto_urls,availability_status,moderation_status,deleted_at,latitude,longitude,created_at,updated_at FROM iklan_barang_bekas.iklan WHERE id=$1";
const SQL_LIST: &str = "SELECT id,seller_id,judul,deskripsi,jenis_barang,jumlah,lokasi_pengambilan,lokasi,region_id,foto_urls,availability_status,moderation_status,deleted_at,latitude,longitude,created_at,updated_at FROM iklan_barang_bekas.iklan WHERE availability_status='tersedia' AND moderation_status='active' AND deleted_at IS NULL ORDER BY created_at DESC LIMIT $1 OFFSET $2";
/// Filter radius (F-1, PRD §5.14.1) — bounding-box (`$3..$6`, memanfaatkan index
/// `(latitude, longitude)`) + Haversine presisi (`$7`=lat pusat, `$8`=lng pusat, `$9`=radius km),
/// formula identik dengan `common_geo::haversine_km()`. Dievaluasi DI DALAM query (bukan hanya
/// bounding-box) supaya `LIMIT`/`OFFSET` benar terhadap hasil yang sudah terpotong radius.
const SQL_LIST_RADIUS: &str = "SELECT id,seller_id,judul,deskripsi,jenis_barang,jumlah,lokasi_pengambilan,lokasi,region_id,foto_urls,availability_status,moderation_status,deleted_at,latitude,longitude,created_at,updated_at \
    FROM iklan_barang_bekas.iklan \
    WHERE availability_status='tersedia' AND moderation_status='active' AND deleted_at IS NULL \
      AND latitude IS NOT NULL AND longitude IS NOT NULL \
      AND latitude BETWEEN $3 AND $4 AND longitude BETWEEN $5 AND $6 \
      AND (2 * 6371 * asin(sqrt( \
            power(sin(radians((latitude - $7) / 2)), 2) + \
            cos(radians($7)) * cos(radians(latitude)) * power(sin(radians((longitude - $8) / 2)), 2) \
          ))) <= $9 \
    ORDER BY (2 * 6371 * asin(sqrt( \
            power(sin(radians((latitude - $7) / 2)), 2) + \
            cos(radians($7)) * cos(radians(latitude)) * power(sin(radians((longitude - $8) / 2)), 2) \
          ))) ASC \
    LIMIT $1 OFFSET $2";
const SQL_CREATE: &str = "INSERT INTO iklan_barang_bekas.iklan (id,seller_id,judul,deskripsi,jenis_barang,jumlah,lokasi_pengambilan,lokasi,region_id,foto_urls,latitude,longitude) VALUES (gen_random_uuid(),$1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11) RETURNING id,seller_id,judul,deskripsi,jenis_barang,jumlah,lokasi_pengambilan,lokasi,region_id,foto_urls,availability_status,moderation_status,deleted_at,latitude,longitude,created_at,updated_at";
const SQL_MARK_TAKEN: &str = "UPDATE iklan_barang_bekas.iklan SET availability_status='sudah_diambil', updated_at=now() WHERE id=$1 AND seller_id=$2 AND availability_status='tersedia'";
const SQL_DELETE: &str = "DELETE FROM iklan_barang_bekas.iklan WHERE id=$1 AND seller_id=$2";
const SQL_UPDATE: &str = r#"
UPDATE iklan_barang_bekas.iklan SET
    judul       = COALESCE($3, judul),
    deskripsi   = COALESCE($4, deskripsi),
    jenis_barang = COALESCE($5, jenis_barang),
    jumlah      = COALESCE($6, jumlah),
    lokasi_pengambilan = COALESCE($7, lokasi_pengambilan),
    lokasi      = COALESCE($8, lokasi),
    region_id   = COALESCE($9, region_id),
    foto_urls   = COALESCE($10, foto_urls),
    is_active   = COALESCE($11, is_active),
    latitude    = COALESCE($12, latitude),
    longitude   = COALESCE($13, longitude),
    updated_at  = now()
WHERE id = $1
  AND seller_id = $2
  AND deleted_at IS NULL
RETURNING id,seller_id,judul,deskripsi,jenis_barang,jumlah,lokasi_pengambilan,lokasi,region_id,foto_urls,availability_status,moderation_status,deleted_at,latitude,longitude,created_at,updated_at
"#;
const SQL_EXISTS: &str = "SELECT EXISTS(SELECT 1 FROM iklan_barang_bekas.iklan WHERE id=$1)";
const SQL_SUSPEND_UPDATE: &str = "UPDATE iklan_barang_bekas.iklan SET moderation_status=$1, updated_at=now() WHERE id=$2 AND deleted_at IS NULL RETURNING id";
const SQL_SUSPEND_INSERT: &str = "INSERT INTO iklan_barang_bekas.iklan_suspension (id,iklan_id,is_permanent,reason,evidence_object_key,expires_at,created_by,created_at) VALUES (gen_random_uuid(),$1,$2,$3,$4,$5,$6,now()) RETURNING id,iklan_id,is_permanent,reason,evidence_object_key,expires_at,created_by,created_at";
const SQL_SOFT_DELETE: &str =
    "UPDATE iklan_barang_bekas.iklan SET deleted_at=now() WHERE id=$1 AND deleted_at IS NULL";
const SQL_EXPIRE_TEMP: &str = "UPDATE iklan_barang_bekas.iklan SET moderation_status='active', updated_at=now() WHERE moderation_status='suspended_temp' AND id IN (SELECT iklan_id FROM iklan_barang_bekas.iklan_suspension WHERE is_permanent=false AND expires_at IS NOT NULL AND expires_at <= now()) AND deleted_at IS NULL";
const SQL_COOLDOWN: &str = "SELECT EXISTS(SELECT 1 FROM iklan_barang_bekas.iklan_suspension WHERE is_permanent=true AND created_at > now() - INTERVAL '3 days' AND iklan_id IN (SELECT id FROM iklan_barang_bekas.iklan WHERE seller_id=$1))";
/// "Iklan Saya" (P4.10) — tidak difilter availability/moderasi (pemilik lihat semua miliknya).
const SQL_LIST_BY_SELLER: &str = "SELECT id,seller_id,judul,deskripsi,jenis_barang,jumlah,lokasi_pengambilan,lokasi,region_id,foto_urls,availability_status,moderation_status,deleted_at,latitude,longitude,created_at,updated_at FROM iklan_barang_bekas.iklan WHERE seller_id=$1 AND deleted_at IS NULL ORDER BY created_at DESC LIMIT $2 OFFSET $3";
/// Batch lookup (P4.11 enrichment) — `WHERE id = ANY($1)`.
const SQL_FIND_BY_IDS: &str = "SELECT id,seller_id,judul,deskripsi,jenis_barang,jumlah,lokasi_pengambilan,lokasi,region_id,foto_urls,availability_status,moderation_status,deleted_at,latitude,longitude,created_at,updated_at FROM iklan_barang_bekas.iklan WHERE id = ANY($1)";

const ADMIN_LIST_SEARCH_ASC: &str = "SELECT id,seller_id,judul,deskripsi,jenis_barang,jumlah,lokasi_pengambilan,lokasi,region_id,foto_urls,availability_status,moderation_status,deleted_at,latitude,longitude,created_at,updated_at,COUNT(*) OVER() AS total_rows FROM iklan_barang_bekas.iklan WHERE moderation_status=$1 AND deleted_at IS NULL AND (judul ILIKE $2 OR seller_id::text ILIKE $2) ORDER BY created_at ASC LIMIT $3 OFFSET $4";
const ADMIN_LIST_SEARCH_DESC: &str = "SELECT id,seller_id,judul,deskripsi,jenis_barang,jumlah,lokasi_pengambilan,lokasi,region_id,foto_urls,availability_status,moderation_status,deleted_at,latitude,longitude,created_at,updated_at,COUNT(*) OVER() AS total_rows FROM iklan_barang_bekas.iklan WHERE moderation_status=$1 AND deleted_at IS NULL AND (judul ILIKE $2 OR seller_id::text ILIKE $2) ORDER BY created_at DESC LIMIT $3 OFFSET $4";
const ADMIN_LIST_ASC: &str = "SELECT id,seller_id,judul,deskripsi,jenis_barang,jumlah,lokasi_pengambilan,lokasi,region_id,foto_urls,availability_status,moderation_status,deleted_at,latitude,longitude,created_at,updated_at,COUNT(*) OVER() AS total_rows FROM iklan_barang_bekas.iklan WHERE moderation_status=$1 AND deleted_at IS NULL ORDER BY created_at ASC LIMIT $2 OFFSET $3";
const ADMIN_LIST_DESC: &str = "SELECT id,seller_id,judul,deskripsi,jenis_barang,jumlah,lokasi_pengambilan,lokasi,region_id,foto_urls,availability_status,moderation_status,deleted_at,latitude,longitude,created_at,updated_at,COUNT(*) OVER() AS total_rows FROM iklan_barang_bekas.iklan WHERE moderation_status=$1 AND deleted_at IS NULL ORDER BY created_at DESC LIMIT $2 OFFSET $3";
const ADMIN_EXPORT_SEARCH: &str = "SELECT id,seller_id,judul,deskripsi,jenis_barang,jumlah,lokasi_pengambilan,lokasi,region_id,foto_urls,availability_status,moderation_status,deleted_at,latitude,longitude,created_at,updated_at FROM iklan_barang_bekas.iklan WHERE moderation_status=$1 AND deleted_at IS NULL AND (judul ILIKE $2 OR seller_id::text ILIKE $2) ORDER BY created_at DESC LIMIT $3";
const ADMIN_EXPORT: &str = "SELECT id,seller_id,judul,deskripsi,jenis_barang,jumlah,lokasi_pengambilan,lokasi,region_id,foto_urls,availability_status,moderation_status,deleted_at,latitude,longitude,created_at,updated_at FROM iklan_barang_bekas.iklan WHERE moderation_status=$1 AND deleted_at IS NULL ORDER BY created_at DESC LIMIT $2";

// ── Bider (F-15, Kelompok 3 Phase 3) ──────────────────────────────────────────
const SQL_BIDER_FIND_BY_ID: &str = "SELECT id,iklan_id,peminat_id,status,sudah_menghubungi,created_at,updated_at FROM iklan_barang_bekas.bider WHERE id=$1";
const SQL_BIDER_CREATE: &str = "INSERT INTO iklan_barang_bekas.bider (id,iklan_id,peminat_id) VALUES (gen_random_uuid(),$1,$2) RETURNING id,iklan_id,peminat_id,status,sudah_menghubungi,created_at,updated_at";
const SQL_BIDER_LIST_FOR_IKLAN: &str = "SELECT id,iklan_id,peminat_id,status,sudah_menghubungi,created_at,updated_at FROM iklan_barang_bekas.bider WHERE iklan_id=$1 ORDER BY created_at DESC";
const SQL_BIDER_HAS_PENDING: &str = "SELECT EXISTS(SELECT 1 FROM iklan_barang_bekas.bider WHERE iklan_id=$1 AND peminat_id=$2 AND status='menunggu')";
/// P3.4: Menunggu→Disetujui, ownership check DI QUERY (`iklan.seller_id`, IDOR→404).
const SQL_BIDER_SETUJUI: &str = "UPDATE iklan_barang_bekas.bider SET status='disetujui', sudah_menghubungi=$1, updated_at=now() WHERE id=$2 AND status='menunggu' AND EXISTS(SELECT 1 FROM iklan_barang_bekas.iklan WHERE id=bider.iklan_id AND seller_id=$3 AND deleted_at IS NULL) RETURNING id,iklan_id,peminat_id,status,sudah_menghubungi,created_at,updated_at";
const SQL_BIDER_SETUJUI_MARK_IKLAN_TAKEN: &str = "UPDATE iklan_barang_bekas.iklan SET availability_status='sudah_diambil', updated_at=now() WHERE id=$1 AND availability_status='tersedia'";
const SQL_BIDER_WITHDRAW_OTHERS: &str = "UPDATE iklan_barang_bekas.bider SET status='withdrawn', updated_at=now() WHERE iklan_id=$1 AND id != $2 AND status='menunggu'";
/// P3.5: kunci baris + baca status LAMA sebelum update (perlu tahu apakah sebelumnya
/// `disetujui` untuk menentukan re-listing) — ownership check DI QUERY.
const SQL_BIDER_LOCK_FOR_WITHDRAW: &str = "SELECT b.id,b.iklan_id,b.peminat_id,b.status,b.sudah_menghubungi,b.created_at,b.updated_at FROM iklan_barang_bekas.bider b WHERE b.id=$1 AND b.status IN ('menunggu','disetujui') AND EXISTS(SELECT 1 FROM iklan_barang_bekas.iklan WHERE id=b.iklan_id AND seller_id=$2 AND deleted_at IS NULL) FOR UPDATE";
const SQL_BIDER_WITHDRAW: &str = "UPDATE iklan_barang_bekas.bider SET status='withdrawn', updated_at=now() WHERE id=$1 RETURNING id,iklan_id,peminat_id,status,sudah_menghubungi,created_at,updated_at";
const SQL_BIDER_WITHDRAW_RELIST_IKLAN: &str = "UPDATE iklan_barang_bekas.iklan SET availability_status='tersedia', updated_at=now() WHERE id=$1";
/// "Bider Saya" (P4.11) — daftar bid milik satu peminat, seluruh iklan.
const SQL_BIDER_LIST_FOR_PEMINAT: &str = "SELECT id,iklan_id,peminat_id,status,sudah_menghubungi,created_at,updated_at FROM iklan_barang_bekas.bider WHERE peminat_id=$1 ORDER BY created_at DESC";
