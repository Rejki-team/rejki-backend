use std::time::Instant;

use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::domain::entity::{
    enrollment_status_name, status_name, CreatedByRole, EnrollmentStatus, IklanPelatihan,
    IklanSuspension, ModerationStatus, PelatihanBadge, PelatihanEnrollment, PelatihanStatus,
};
use crate::domain::repository::{
    AdminListParams, AdminListResult, CreatePelatihanParams, IklanPelatihanRepository, ListParams,
    PatchPelatihanParams, UpdatePelatihanParams,
};

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

// ── Row mappers ──────────────────────────────────────────────────────────

fn row_to_entity(r: &sqlx::postgres::PgRow) -> IklanPelatihan {
    IklanPelatihan {
        id: r.get("id"),
        poster_id: r.get("poster_id"),
        judul: r.get("judul"),
        penyelenggara: r.get("penyelenggara"),
        deskripsi: r.get("deskripsi"),
        lokasi: r.get("lokasi"),
        region_id: r.get("region_id"),
        harga: r.get("harga"),
        tanggal_mulai: r.get("tanggal_mulai"),
        tanggal_selesai: r.get("tanggal_selesai"),
        foto_urls: r
            .get::<Option<Vec<String>>, _>("foto_urls")
            .unwrap_or_default(),
        is_active: r.get("is_active"),
        moderation_status: ModerationStatus::parse(&r.get::<String, _>("moderation_status"))
            .unwrap_or_default(),
        status: PelatihanStatus::parse(&r.get::<String, _>("status")).unwrap_or_default(),
        created_by_role: CreatedByRole::parse(&r.get::<String, _>("created_by_role"))
            .unwrap_or_default(),
        jumlah_peserta: r.get("jumlah_peserta"),
        reviewed_by: r.get("reviewed_by"),
        review_note: r.get("review_note"),
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

fn row_to_enrollment(r: &sqlx::postgres::PgRow) -> PelatihanEnrollment {
    PelatihanEnrollment {
        id: r.get("id"),
        pelatihan_id: r.get("pelatihan_id"),
        user_id: r.get("user_id"),
        bukti_transfer_object_key: r.get("bukti_transfer_object_key"),
        status: EnrollmentStatus::parse(&r.get::<String, _>("status")).unwrap_or_default(),
        reviewed_by: r.get("reviewed_by"),
        review_note: r.get("review_note"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    }
}

fn row_to_badge(r: &sqlx::postgres::PgRow) -> PelatihanBadge {
    PelatihanBadge {
        id: r.get("id"),
        pelatihan_id: r.get("pelatihan_id"),
        user_id: r.get("user_id"),
        sertifikat_object_key: r.get("sertifikat_object_key"),
        approved_at: r.get("approved_at"),
        status: EnrollmentStatus::parse(&r.get::<String, _>("status")).unwrap_or_default(),
        reviewed_by: r.get("reviewed_by"),
        review_note: r.get("review_note"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    }
}

/// Full column list for iklan_pelatihan.iklan (keep in sync with schema).
const _PELATIHAN_COLS: &str = "id,poster_id,judul,penyelenggara,deskripsi,lokasi,region_id,harga,tanggal_mulai,tanggal_selesai,foto_urls,is_active,moderation_status,status,created_by_role,jumlah_peserta,reviewed_by,review_note,deleted_at,created_at,updated_at";

// Keep original static string approach — sqlx 0.9 requires literal SQL strings.
// Every query uses the full column list inline.

macro_rules! pelatihan_cols {
    () => {
        "id,poster_id,judul,penyelenggara,deskripsi,lokasi,region_id,harga,tanggal_mulai,tanggal_selesai,foto_urls,is_active,moderation_status,status,created_by_role,jumlah_peserta,reviewed_by,review_note,deleted_at,created_at,updated_at"
    };
}

macro_rules! enrollment_cols {
    () => {
        "id,pelatihan_id,user_id,bukti_transfer_object_key,status,reviewed_by,review_note,created_at,updated_at"
    };
}

macro_rules! badge_cols {
    () => {
        "id,pelatihan_id,user_id,sertifikat_object_key,approved_at,status,reviewed_by,review_note,created_at,updated_at"
    };
}

impl IklanPelatihanRepository for PgIklanPelatihanRepository {
    // ── Core CRUD ────────────────────────────────────────────────────────

    async fn find_by_id(&self, id: Uuid) -> Result<Option<IklanPelatihan>, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query(concat!(
            "SELECT ",
            pelatihan_cols!(),
            " FROM iklan_pelatihan.iklan WHERE id=$1"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        warn_slow!(t, "iklan_pelatihan.find_by_id");
        Ok(r.as_ref().map(row_to_entity))
    }

    async fn list(&self, limit: i64, offset: i64) -> Result<Vec<IklanPelatihan>, anyhow::Error> {
        let t = Instant::now();
        let rows = sqlx::query(concat!(
            "SELECT ",
            pelatihan_cols!(),
            " FROM iklan_pelatihan.iklan WHERE is_active=true AND moderation_status='active' AND deleted_at IS NULL AND status IN ('verifikasi_diterima','pelatihan_belum_dimulai','pelatihan_berjalan','pelatihan_selesai') ORDER BY created_at DESC LIMIT $1 OFFSET $2"
        ))
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        warn_slow!(t, "iklan_pelatihan.list");
        Ok(rows.iter().map(row_to_entity).collect())
    }

    async fn create(
        &self,
        params: CreatePelatihanParams<'_>,
    ) -> Result<IklanPelatihan, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query(concat!(
            "INSERT INTO iklan_pelatihan.iklan (id,poster_id,judul,penyelenggara,deskripsi,lokasi,region_id,harga,tanggal_mulai,tanggal_selesai,created_by_role,status,jumlah_peserta) VALUES (gen_random_uuid(),$1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12) RETURNING ",
            pelatihan_cols!()
        ))
        .bind(params.poster_id)
        .bind(params.judul)
        .bind(params.penyelenggara)
        .bind(params.deskripsi)
        .bind(params.lokasi)
        .bind(params.region_id)
        .bind(params.harga)
        .bind(params.tanggal_mulai)
        .bind(params.tanggal_selesai)
        .bind(params.created_by_role)
        .bind(params.initial_status)
        .bind(params.jumlah_peserta)
        .fetch_one(&self.pool)
        .await?;
        warn_slow!(t, "iklan_pelatihan.create");
        Ok(row_to_entity(&r))
    }

    async fn delete(&self, id: Uuid, poster_id: Uuid) -> Result<bool, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query("DELETE FROM iklan_pelatihan.iklan WHERE id=$1 AND poster_id=$2")
            .bind(id)
            .bind(poster_id)
            .execute(&self.pool)
            .await?;
        warn_slow!(t, "iklan_pelatihan.delete");
        Ok(r.rows_affected() > 0)
    }

    async fn exists(&self, id: Uuid) -> Result<bool, anyhow::Error> {
        let t = Instant::now();
        let r: Option<bool> =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM iklan_pelatihan.iklan WHERE id=$1)")
                .bind(id)
                .fetch_one(&self.pool)
                .await?;
        warn_slow!(t, "iklan_pelatihan.exists");
        Ok(r.unwrap_or(false))
    }

    // ── PATCH (partial update by owner) ─────────────────────────────────

    async fn update(
        &self,
        id: Uuid,
        poster_id: Uuid,
        params: PatchPelatihanParams,
    ) -> Result<Option<IklanPelatihan>, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query(concat!(
            "UPDATE iklan_pelatihan.iklan SET ",
            "judul=COALESCE($3,judul),",
            "penyelenggara=COALESCE($4,penyelenggara),",
            "deskripsi=COALESCE($5,deskripsi),",
            "lokasi=COALESCE($6,lokasi),",
            "region_id=COALESCE($7,region_id),",
            "harga=COALESCE($8,harga),",
            "tanggal_mulai=COALESCE($9,tanggal_mulai),",
            "tanggal_selesai=COALESCE($10,tanggal_selesai),",
            "foto_urls=COALESCE($11,foto_urls),",
            "jumlah_peserta=COALESCE($12,jumlah_peserta),",
            "is_active=COALESCE($13,is_active),",
            "updated_at=now() ",
            "WHERE id=$1 AND poster_id=$2 AND deleted_at IS NULL RETURNING ",
            pelatihan_cols!()
        ))
        .bind(id)
        .bind(poster_id)
        .bind(&params.judul)
        .bind(&params.penyelenggara)
        .bind(&params.deskripsi)
        .bind(&params.lokasi)
        .bind(&params.region_id)
        .bind(params.harga)
        .bind(params.tanggal_mulai)
        .bind(params.tanggal_selesai)
        .bind(&params.foto_urls)
        .bind(params.jumlah_peserta)
        .bind(params.is_active)
        .fetch_optional(&self.pool)
        .await?;
        warn_slow!(t, "iklan_pelatihan.update");
        Ok(r.as_ref().map(row_to_entity))
    }

    // ── Admin pelatihan listing (7-stage status) ─────────────────────────

    async fn admin_pelatihan_list(
        &self,
        params: ListParams,
    ) -> Result<AdminListResult<IklanPelatihan>, anyhow::Error> {
        let t = Instant::now();
        let filter_value = params
            .filter_value
            .as_deref()
            .unwrap_or(status_name::VERIFIKASI_TERTUNDA);
        let q_pattern = params.q.as_deref().map(|s| format!("%{}%", s.trim()));
        let asc = matches!(params.sort_dir.as_deref(), Some("asc"));

        let rows = match (&q_pattern, asc) {
            (Some(q), true) => sqlx::query(concat!(
                "SELECT ", pelatihan_cols!(), ", COUNT(*) OVER() AS total_rows FROM iklan_pelatihan.iklan WHERE deleted_at IS NULL AND status=$1 AND (judul ILIKE $2 OR poster_id::text ILIKE $2 OR penyelenggara ILIKE $2) ORDER BY created_at ASC LIMIT $3 OFFSET $4"
            ))
            .bind(filter_value).bind(q.as_str()).bind(params.limit).bind(params.offset)
            .fetch_all(&self.pool).await?,
            (Some(q), false) => sqlx::query(concat!(
                "SELECT ", pelatihan_cols!(), ", COUNT(*) OVER() AS total_rows FROM iklan_pelatihan.iklan WHERE deleted_at IS NULL AND status=$1 AND (judul ILIKE $2 OR poster_id::text ILIKE $2 OR penyelenggara ILIKE $2) ORDER BY created_at DESC LIMIT $3 OFFSET $4"
            ))
            .bind(filter_value).bind(q.as_str()).bind(params.limit).bind(params.offset)
            .fetch_all(&self.pool).await?,
            (None, true) => sqlx::query(concat!(
                "SELECT ", pelatihan_cols!(), ", COUNT(*) OVER() AS total_rows FROM iklan_pelatihan.iklan WHERE deleted_at IS NULL AND status=$1 ORDER BY created_at ASC LIMIT $2 OFFSET $3"
            ))
            .bind(filter_value).bind(params.limit).bind(params.offset)
            .fetch_all(&self.pool).await?,
            (None, false) => sqlx::query(concat!(
                "SELECT ", pelatihan_cols!(), ", COUNT(*) OVER() AS total_rows FROM iklan_pelatihan.iklan WHERE deleted_at IS NULL AND status=$1 ORDER BY created_at DESC LIMIT $2 OFFSET $3"
            ))
            .bind(filter_value).bind(params.limit).bind(params.offset)
            .fetch_all(&self.pool).await?,
        };

        let total: i64 = rows
            .first()
            .map_or(0, |r| r.try_get::<i64, _>("total_rows").unwrap_or(0));
        warn_slow!(t, "iklan_pelatihan.admin_pelatihan_list");
        Ok(AdminListResult {
            items: rows.iter().map(row_to_entity).collect(),
            total,
        })
    }

    async fn admin_pelatihan_list_all(
        &self,
        params: ListParams,
    ) -> Result<Vec<IklanPelatihan>, anyhow::Error> {
        let t = Instant::now();
        let filter_value = params
            .filter_value
            .as_deref()
            .unwrap_or(status_name::VERIFIKASI_TERTUNDA);
        let q_pattern = params.q.as_deref().map(|s| format!("%{}%", s.trim()));

        let rows = match &q_pattern {
            Some(q) => sqlx::query(concat!(
                "SELECT ", pelatihan_cols!(), " FROM iklan_pelatihan.iklan WHERE deleted_at IS NULL AND status=$1 AND (judul ILIKE $2 OR poster_id::text ILIKE $2 OR penyelenggara ILIKE $2) ORDER BY created_at DESC LIMIT $3"
            ))
            .bind(filter_value).bind(q.as_str()).bind(params.limit)
            .fetch_all(&self.pool).await?,
            None => sqlx::query(concat!(
                "SELECT ", pelatihan_cols!(), " FROM iklan_pelatihan.iklan WHERE deleted_at IS NULL AND status=$1 ORDER BY created_at DESC LIMIT $2"
            ))
            .bind(filter_value).bind(params.limit)
            .fetch_all(&self.pool).await?,
        };
        warn_slow!(t, "iklan_pelatihan.admin_pelatihan_list_all");
        Ok(rows.iter().map(row_to_entity).collect())
    }

    async fn update_pelatihan(
        &self,
        params: UpdatePelatihanParams<'_>,
    ) -> Result<Option<IklanPelatihan>, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query(concat!(
            "UPDATE iklan_pelatihan.iklan SET judul=$3,penyelenggara=$4,deskripsi=$5,lokasi=$6,region_id=$7,harga=$8,tanggal_mulai=$9,tanggal_selesai=$10,jumlah_peserta=$11,updated_at=now() WHERE id=$1 AND poster_id=$2 AND deleted_at IS NULL RETURNING ",
            pelatihan_cols!()
        ))
        .bind(params.id)
        .bind(params.poster_id)
        .bind(params.judul)
        .bind(params.penyelenggara)
        .bind(params.deskripsi)
        .bind(params.lokasi)
        .bind(params.region_id)
        .bind(params.harga)
        .bind(params.tanggal_mulai)
        .bind(params.tanggal_selesai)
        .bind(params.jumlah_peserta)
        .fetch_optional(&self.pool)
        .await?;
        warn_slow!(t, "iklan_pelatihan.update_pelatihan");
        Ok(r.as_ref().map(row_to_entity))
    }

    async fn review_pelatihan(
        &self,
        id: Uuid,
        approved: bool,
        review_note: Option<&str>,
        reviewer_id: Uuid,
    ) -> Result<Option<IklanPelatihan>, anyhow::Error> {
        let t = Instant::now();
        let new_status = if approved {
            status_name::VERIFIKASI_DITERIMA
        } else {
            status_name::VERIFIKASI_DITOLAK
        };
        let r = sqlx::query(concat!(
            "UPDATE iklan_pelatihan.iklan SET status=$2, reviewed_by=$3, review_note=$4, updated_at=now() WHERE id=$1 AND status IN ('verifikasi_tertunda','verifikasi_dalam_proses') AND deleted_at IS NULL RETURNING ",
            pelatihan_cols!()
        ))
        .bind(id)
        .bind(new_status)
        .bind(reviewer_id)
        .bind(review_note)
        .fetch_optional(&self.pool)
        .await?;
        warn_slow!(t, "iklan_pelatihan.review_pelatihan");
        Ok(r.as_ref().map(row_to_entity))
    }

    async fn soft_delete_pelatihan(
        &self,
        id: Uuid,
        poster_id: Uuid,
    ) -> Result<bool, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query(
            "UPDATE iklan_pelatihan.iklan SET deleted_at=now() WHERE id=$1 AND poster_id=$2 AND deleted_at IS NULL",
        )
        .bind(id)
        .bind(poster_id)
        .execute(&self.pool)
        .await?;
        warn_slow!(t, "iklan_pelatihan.soft_delete_pelatihan");
        Ok(r.rows_affected() > 0)
    }

    // ── Suspension (existing) ────────────────────────────────────────────

    async fn admin_list(
        &self,
        params: AdminListParams,
    ) -> Result<AdminListResult<IklanPelatihan>, anyhow::Error> {
        let t = Instant::now();
        let status = params.moderation_status.as_deref().unwrap_or("active");
        let q_pattern = params.q.as_deref().map(|s| format!("%{}%", s.trim()));
        let asc = matches!(params.sort_dir.as_deref(), Some("asc"));

        let rows = match (&q_pattern, asc) {
            (Some(q), true) => sqlx::query(concat!(
                "SELECT ", pelatihan_cols!(), ", COUNT(*) OVER() AS total_rows FROM iklan_pelatihan.iklan WHERE moderation_status=$1 AND deleted_at IS NULL AND (judul ILIKE $2 OR poster_id::text ILIKE $2 OR penyelenggara ILIKE $2) ORDER BY created_at ASC LIMIT $3 OFFSET $4"
            ))
            .bind(status).bind(q.as_str()).bind(params.limit).bind(params.offset)
            .fetch_all(&self.pool).await?,
            (Some(q), false) => sqlx::query(concat!(
                "SELECT ", pelatihan_cols!(), ", COUNT(*) OVER() AS total_rows FROM iklan_pelatihan.iklan WHERE moderation_status=$1 AND deleted_at IS NULL AND (judul ILIKE $2 OR poster_id::text ILIKE $2 OR penyelenggara ILIKE $2) ORDER BY created_at DESC LIMIT $3 OFFSET $4"
            ))
            .bind(status).bind(q.as_str()).bind(params.limit).bind(params.offset)
            .fetch_all(&self.pool).await?,
            (None, true) => sqlx::query(concat!(
                "SELECT ", pelatihan_cols!(), ", COUNT(*) OVER() AS total_rows FROM iklan_pelatihan.iklan WHERE moderation_status=$1 AND deleted_at IS NULL ORDER BY created_at ASC LIMIT $2 OFFSET $3"
            ))
            .bind(status).bind(params.limit).bind(params.offset)
            .fetch_all(&self.pool).await?,
            (None, false) => sqlx::query(concat!(
                "SELECT ", pelatihan_cols!(), ", COUNT(*) OVER() AS total_rows FROM iklan_pelatihan.iklan WHERE moderation_status=$1 AND deleted_at IS NULL ORDER BY created_at DESC LIMIT $2 OFFSET $3"
            ))
            .bind(status).bind(params.limit).bind(params.offset)
            .fetch_all(&self.pool).await?,
        };

        let total: i64 = rows
            .first()
            .map_or(0, |r| r.try_get::<i64, _>("total_rows").unwrap_or(0));
        warn_slow!(t, "iklan_pelatihan.admin_list");
        Ok(AdminListResult {
            items: rows.iter().map(row_to_entity).collect(),
            total,
        })
    }

    async fn admin_list_all(
        &self,
        params: AdminListParams,
    ) -> Result<Vec<IklanPelatihan>, anyhow::Error> {
        let t = Instant::now();
        let status = params.moderation_status.as_deref().unwrap_or("active");
        let q_pattern = params.q.as_deref().map(|s| format!("%{}%", s.trim()));

        let rows = match &q_pattern {
            Some(q) => sqlx::query(concat!(
                "SELECT ", pelatihan_cols!(), " FROM iklan_pelatihan.iklan WHERE moderation_status=$1 AND deleted_at IS NULL AND (judul ILIKE $2 OR poster_id::text ILIKE $2 OR penyelenggara ILIKE $2) ORDER BY created_at DESC LIMIT $3"
            ))
            .bind(status).bind(q.as_str()).bind(params.limit)
            .fetch_all(&self.pool).await?,
            None => sqlx::query(concat!(
                "SELECT ", pelatihan_cols!(), " FROM iklan_pelatihan.iklan WHERE moderation_status=$1 AND deleted_at IS NULL ORDER BY created_at DESC LIMIT $2"
            ))
            .bind(status).bind(params.limit)
            .fetch_all(&self.pool).await?,
        };
        warn_slow!(t, "iklan_pelatihan.admin_list_all");
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
            let updated = sqlx::query("UPDATE iklan_pelatihan.iklan SET moderation_status=$1, updated_at=now() WHERE id=$2 AND deleted_at IS NULL RETURNING id")
                .bind(new_status).bind(iklan_id).fetch_optional(&self.pool).await?;
            warn_slow!(t, "iklan_pelatihan.suspend_update");

            if updated.is_some() {
                let s = sqlx::query("INSERT INTO iklan_pelatihan.iklan_suspension (id,iklan_id,is_permanent,reason,evidence_object_key,expires_at,created_by,created_at) VALUES (gen_random_uuid(),$1,$2,$3,$4,$5,$6,now()) RETURNING id,iklan_id,is_permanent,reason,evidence_object_key,expires_at,created_by,created_at")
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
            "UPDATE iklan_pelatihan.iklan SET deleted_at=now() WHERE id=$1 AND deleted_at IS NULL",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;
        warn_slow!(t, "iklan_pelatihan.soft_delete");
        Ok(r.rows_affected() > 0)
    }

    async fn expire_temporary_suspensions(&self) -> Result<u64, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query(
            "UPDATE iklan_pelatihan.iklan SET moderation_status='active', updated_at=now() WHERE moderation_status='suspended_temp' AND id IN (SELECT iklan_id FROM iklan_pelatihan.iklan_suspension WHERE is_permanent=false AND expires_at IS NOT NULL AND expires_at <= now()) AND deleted_at IS NULL",
        )
        .execute(&self.pool)
        .await?;
        warn_slow!(t, "iklan_pelatihan.expire_temporary_suspensions");
        Ok(r.rows_affected())
    }

    async fn is_poster_in_cooldown(&self, poster_id: Uuid) -> Result<bool, anyhow::Error> {
        let t = Instant::now();
        let r: Option<bool> = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM iklan_pelatihan.iklan_suspension WHERE is_permanent=true AND created_at > now() - INTERVAL '3 days' AND iklan_id IN (SELECT id FROM iklan_pelatihan.iklan WHERE poster_id=$1))",
        )
        .bind(poster_id)
        .fetch_one(&self.pool)
        .await?;
        warn_slow!(t, "iklan_pelatihan.is_poster_in_cooldown");
        Ok(r.unwrap_or(false))
    }

    // ── Enrollment ───────────────────────────────────────────────────────

    async fn create_enrollment(
        &self,
        pelatihan_id: Uuid,
        user_id: Uuid,
    ) -> Result<PelatihanEnrollment, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query(concat!(
            "INSERT INTO iklan_pelatihan.pelatihan_enrollment (id,pelatihan_id,user_id) VALUES (gen_random_uuid(),$1,$2) RETURNING ",
            enrollment_cols!()
        ))
        .bind(pelatihan_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;
        warn_slow!(t, "iklan_pelatihan.create_enrollment");
        Ok(row_to_enrollment(&r))
    }

    async fn find_enrollment_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<PelatihanEnrollment>, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query(concat!(
            "SELECT ",
            enrollment_cols!(),
            " FROM iklan_pelatihan.pelatihan_enrollment WHERE id=$1"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        warn_slow!(t, "iklan_pelatihan.find_enrollment_by_id");
        Ok(r.as_ref().map(row_to_enrollment))
    }

    async fn commit_enrollment_bukti(
        &self,
        id: Uuid,
        user_id: Uuid,
        object_key: &str,
    ) -> Result<Option<PelatihanEnrollment>, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query(concat!(
            "UPDATE iklan_pelatihan.pelatihan_enrollment SET bukti_transfer_object_key=$3, updated_at=now() WHERE id=$1 AND user_id=$2 AND bukti_transfer_object_key IS NULL RETURNING ",
            enrollment_cols!()
        ))
        .bind(id)
        .bind(user_id)
        .bind(object_key)
        .fetch_optional(&self.pool)
        .await?;
        warn_slow!(t, "iklan_pelatihan.commit_enrollment_bukti");
        Ok(r.as_ref().map(row_to_enrollment))
    }

    async fn admin_enrollment_list(
        &self,
        params: ListParams,
    ) -> Result<AdminListResult<PelatihanEnrollment>, anyhow::Error> {
        let t = Instant::now();
        let filter_value = params
            .filter_value
            .as_deref()
            .unwrap_or(enrollment_status_name::PENDING);
        let q_pattern = params.q.as_deref().map(|s| format!("%{}%", s.trim()));
        let asc = matches!(params.sort_dir.as_deref(), Some("asc"));

        let rows = match (&q_pattern, asc) {
            (Some(q), true) => sqlx::query(concat!(
                "SELECT e.", enrollment_cols!(), ", COUNT(*) OVER() AS total_rows FROM iklan_pelatihan.pelatihan_enrollment e JOIN iklan_pelatihan.iklan i ON e.pelatihan_id = i.id WHERE e.status=$1 AND (i.judul ILIKE $2 OR e.pelatihan_id::text ILIKE $2 OR i.penyelenggara ILIKE $2) ORDER BY e.created_at ASC LIMIT $3 OFFSET $4"
            ))
            .bind(filter_value).bind(q.as_str()).bind(params.limit).bind(params.offset)
            .fetch_all(&self.pool).await?,
            (Some(q), false) => sqlx::query(concat!(
                "SELECT e.", enrollment_cols!(), ", COUNT(*) OVER() AS total_rows FROM iklan_pelatihan.pelatihan_enrollment e JOIN iklan_pelatihan.iklan i ON e.pelatihan_id = i.id WHERE e.status=$1 AND (i.judul ILIKE $2 OR e.pelatihan_id::text ILIKE $2 OR i.penyelenggara ILIKE $2) ORDER BY e.created_at DESC LIMIT $3 OFFSET $4"
            ))
            .bind(filter_value).bind(q.as_str()).bind(params.limit).bind(params.offset)
            .fetch_all(&self.pool).await?,
            (None, true) => sqlx::query(concat!(
                "SELECT e.", enrollment_cols!(), ", COUNT(*) OVER() AS total_rows FROM iklan_pelatihan.pelatihan_enrollment e JOIN iklan_pelatihan.iklan i ON e.pelatihan_id = i.id WHERE e.status=$1 ORDER BY e.created_at ASC LIMIT $2 OFFSET $3"
            ))
            .bind(filter_value).bind(params.limit).bind(params.offset)
            .fetch_all(&self.pool).await?,
            (None, false) => sqlx::query(concat!(
                "SELECT e.", enrollment_cols!(), ", COUNT(*) OVER() AS total_rows FROM iklan_pelatihan.pelatihan_enrollment e JOIN iklan_pelatihan.iklan i ON e.pelatihan_id = i.id WHERE e.status=$1 ORDER BY e.created_at DESC LIMIT $2 OFFSET $3"
            ))
            .bind(filter_value).bind(params.limit).bind(params.offset)
            .fetch_all(&self.pool).await?,
        };

        let total: i64 = rows
            .first()
            .map_or(0, |r| r.try_get::<i64, _>("total_rows").unwrap_or(0));
        warn_slow!(t, "iklan_pelatihan.admin_enrollment_list");
        Ok(AdminListResult {
            items: rows.iter().map(row_to_enrollment).collect(),
            total,
        })
    }

    async fn admin_enrollment_list_all(
        &self,
        params: ListParams,
    ) -> Result<Vec<PelatihanEnrollment>, anyhow::Error> {
        let t = Instant::now();
        let filter_value = params
            .filter_value
            .as_deref()
            .unwrap_or(enrollment_status_name::PENDING);
        let q_pattern = params.q.as_deref().map(|s| format!("%{}%", s.trim()));

        let rows = match &q_pattern {
            Some(q) => sqlx::query(concat!(
                "SELECT e.", enrollment_cols!(), " FROM iklan_pelatihan.pelatihan_enrollment e JOIN iklan_pelatihan.iklan i ON e.pelatihan_id = i.id WHERE e.status=$1 AND (i.judul ILIKE $2 OR e.pelatihan_id::text ILIKE $2 OR i.penyelenggara ILIKE $2) ORDER BY e.created_at DESC LIMIT $3"
            ))
            .bind(filter_value).bind(q.as_str()).bind(params.limit)
            .fetch_all(&self.pool).await?,
            None => sqlx::query(concat!(
                "SELECT e.", enrollment_cols!(), " FROM iklan_pelatihan.pelatihan_enrollment e JOIN iklan_pelatihan.iklan i ON e.pelatihan_id = i.id WHERE e.status=$1 ORDER BY e.created_at DESC LIMIT $2"
            ))
            .bind(filter_value).bind(params.limit)
            .fetch_all(&self.pool).await?,
        };
        warn_slow!(t, "iklan_pelatihan.admin_enrollment_list_all");
        Ok(rows.iter().map(row_to_enrollment).collect())
    }

    async fn review_enrollment(
        &self,
        id: Uuid,
        approved: bool,
        review_note: Option<&str>,
        reviewer_id: Uuid,
    ) -> Result<Option<PelatihanEnrollment>, anyhow::Error> {
        let t = Instant::now();
        let new_status = if approved {
            enrollment_status_name::APPROVED
        } else {
            enrollment_status_name::REJECTED
        };
        let r = sqlx::query(concat!(
            "UPDATE iklan_pelatihan.pelatihan_enrollment SET status=$2, reviewed_by=$3, review_note=$4, updated_at=now() WHERE id=$1 AND status IN ('pending','in_review') RETURNING ",
            enrollment_cols!()
        ))
        .bind(id)
        .bind(new_status)
        .bind(reviewer_id)
        .bind(review_note)
        .fetch_optional(&self.pool)
        .await?;
        warn_slow!(t, "iklan_pelatihan.review_enrollment");
        Ok(r.as_ref().map(row_to_enrollment))
    }

    // ── Badge ────────────────────────────────────────────────────────────

    async fn create_badge(
        &self,
        pelatihan_id: Uuid,
        user_id: Uuid,
    ) -> Result<PelatihanBadge, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query(concat!(
            "INSERT INTO iklan_pelatihan.pelatihan_badge (id,pelatihan_id,user_id) VALUES (gen_random_uuid(),$1,$2) RETURNING ",
            badge_cols!()
        ))
        .bind(pelatihan_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;
        warn_slow!(t, "iklan_pelatihan.create_badge");
        Ok(row_to_badge(&r))
    }

    async fn find_badge_by_id(&self, id: Uuid) -> Result<Option<PelatihanBadge>, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query(concat!(
            "SELECT ",
            badge_cols!(),
            " FROM iklan_pelatihan.pelatihan_badge WHERE id=$1"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        warn_slow!(t, "iklan_pelatihan.find_badge_by_id");
        Ok(r.as_ref().map(row_to_badge))
    }

    async fn commit_badge_sertifikat(
        &self,
        id: Uuid,
        user_id: Uuid,
        object_key: &str,
    ) -> Result<Option<PelatihanBadge>, anyhow::Error> {
        let t = Instant::now();
        let r = sqlx::query(concat!(
            "UPDATE iklan_pelatihan.pelatihan_badge SET sertifikat_object_key=$3, updated_at=now() WHERE id=$1 AND user_id=$2 AND sertifikat_object_key IS NULL RETURNING ",
            badge_cols!()
        ))
        .bind(id)
        .bind(user_id)
        .bind(object_key)
        .fetch_optional(&self.pool)
        .await?;
        warn_slow!(t, "iklan_pelatihan.commit_badge_sertifikat");
        Ok(r.as_ref().map(row_to_badge))
    }

    async fn admin_badge_list(
        &self,
        params: ListParams,
    ) -> Result<AdminListResult<PelatihanBadge>, anyhow::Error> {
        let t = Instant::now();
        let filter_value = params
            .filter_value
            .as_deref()
            .unwrap_or(enrollment_status_name::PENDING);
        let q_pattern = params.q.as_deref().map(|s| format!("%{}%", s.trim()));
        let asc = matches!(params.sort_dir.as_deref(), Some("asc"));

        let rows = match (&q_pattern, asc) {
            (Some(q), true) => sqlx::query(concat!(
                "SELECT b.", badge_cols!(), ", COUNT(*) OVER() AS total_rows FROM iklan_pelatihan.pelatihan_badge b JOIN iklan_pelatihan.iklan i ON b.pelatihan_id = i.id WHERE b.status=$1 AND (i.judul ILIKE $2 OR b.pelatihan_id::text ILIKE $2 OR i.penyelenggara ILIKE $2) ORDER BY b.created_at ASC LIMIT $3 OFFSET $4"
            ))
            .bind(filter_value).bind(q.as_str()).bind(params.limit).bind(params.offset)
            .fetch_all(&self.pool).await?,
            (Some(q), false) => sqlx::query(concat!(
                "SELECT b.", badge_cols!(), ", COUNT(*) OVER() AS total_rows FROM iklan_pelatihan.pelatihan_badge b JOIN iklan_pelatihan.iklan i ON b.pelatihan_id = i.id WHERE b.status=$1 AND (i.judul ILIKE $2 OR b.pelatihan_id::text ILIKE $2 OR i.penyelenggara ILIKE $2) ORDER BY b.created_at DESC LIMIT $3 OFFSET $4"
            ))
            .bind(filter_value).bind(q.as_str()).bind(params.limit).bind(params.offset)
            .fetch_all(&self.pool).await?,
            (None, true) => sqlx::query(concat!(
                "SELECT b.", badge_cols!(), ", COUNT(*) OVER() AS total_rows FROM iklan_pelatihan.pelatihan_badge b JOIN iklan_pelatihan.iklan i ON b.pelatihan_id = i.id WHERE b.status=$1 ORDER BY b.created_at ASC LIMIT $2 OFFSET $3"
            ))
            .bind(filter_value).bind(params.limit).bind(params.offset)
            .fetch_all(&self.pool).await?,
            (None, false) => sqlx::query(concat!(
                "SELECT b.", badge_cols!(), ", COUNT(*) OVER() AS total_rows FROM iklan_pelatihan.pelatihan_badge b JOIN iklan_pelatihan.iklan i ON b.pelatihan_id = i.id WHERE b.status=$1 ORDER BY b.created_at DESC LIMIT $2 OFFSET $3"
            ))
            .bind(filter_value).bind(params.limit).bind(params.offset)
            .fetch_all(&self.pool).await?,
        };

        let total: i64 = rows
            .first()
            .map_or(0, |r| r.try_get::<i64, _>("total_rows").unwrap_or(0));
        warn_slow!(t, "iklan_pelatihan.admin_badge_list");
        Ok(AdminListResult {
            items: rows.iter().map(row_to_badge).collect(),
            total,
        })
    }

    async fn admin_badge_list_all(
        &self,
        params: ListParams,
    ) -> Result<Vec<PelatihanBadge>, anyhow::Error> {
        let t = Instant::now();
        let filter_value = params
            .filter_value
            .as_deref()
            .unwrap_or(enrollment_status_name::PENDING);
        let q_pattern = params.q.as_deref().map(|s| format!("%{}%", s.trim()));

        let rows = match &q_pattern {
            Some(q) => sqlx::query(concat!(
                "SELECT b.", badge_cols!(), " FROM iklan_pelatihan.pelatihan_badge b JOIN iklan_pelatihan.iklan i ON b.pelatihan_id = i.id WHERE b.status=$1 AND (i.judul ILIKE $2 OR b.pelatihan_id::text ILIKE $2 OR i.penyelenggara ILIKE $2) ORDER BY b.created_at DESC LIMIT $3"
            ))
            .bind(filter_value).bind(q.as_str()).bind(params.limit)
            .fetch_all(&self.pool).await?,
            None => sqlx::query(concat!(
                "SELECT b.", badge_cols!(), " FROM iklan_pelatihan.pelatihan_badge b JOIN iklan_pelatihan.iklan i ON b.pelatihan_id = i.id WHERE b.status=$1 ORDER BY b.created_at DESC LIMIT $2"
            ))
            .bind(filter_value).bind(params.limit)
            .fetch_all(&self.pool).await?,
        };
        warn_slow!(t, "iklan_pelatihan.admin_badge_list_all");
        Ok(rows.iter().map(row_to_badge).collect())
    }

    async fn review_badge(
        &self,
        id: Uuid,
        approved: bool,
        review_note: Option<&str>,
        reviewer_id: Uuid,
    ) -> Result<Option<PelatihanBadge>, anyhow::Error> {
        let t = Instant::now();
        let new_status = if approved {
            enrollment_status_name::APPROVED
        } else {
            enrollment_status_name::REJECTED
        };
        if approved {
            let r = sqlx::query(concat!(
                "UPDATE iklan_pelatihan.pelatihan_badge SET status=$2, reviewed_by=$3, review_note=$4, approved_at=now(), updated_at=now() WHERE id=$1 AND status IN ('pending','in_review') RETURNING ",
                badge_cols!()
            ))
            .bind(id)
            .bind(new_status)
            .bind(reviewer_id)
            .bind(review_note)
            .fetch_optional(&self.pool)
            .await?;
            warn_slow!(t, "iklan_pelatihan.review_badge");
            Ok(r.as_ref().map(row_to_badge))
        } else {
            let r = sqlx::query(concat!(
                "UPDATE iklan_pelatihan.pelatihan_badge SET status=$2, reviewed_by=$3, review_note=$4, updated_at=now() WHERE id=$1 AND status IN ('pending','in_review') RETURNING ",
                badge_cols!()
            ))
            .bind(id)
            .bind(new_status)
            .bind(reviewer_id)
            .bind(review_note)
            .fetch_optional(&self.pool)
            .await?;
            warn_slow!(t, "iklan_pelatihan.review_badge_reject");
            Ok(r.as_ref().map(row_to_badge))
        }
    }
}
