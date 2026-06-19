use std::time::Instant;

use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::domain::entity::{
    DocumentAccessAction, KycSubmission, KycSubmissionStatus, RekeningInfo, UserProfile,
};
use crate::domain::repository::{
    AdminKycListParams, AdminKycListResult, AdminKycRow, TxUserRepository, UpdateProfileParams,
    UserRepository,
};

/// Decrypt phone dengan fallback ke plaintext lama. Gagal → warn + None.
fn decrypt_phone(encrypted: Option<&str>, plaintext_fallback: Option<String>) -> Option<String> {
    let decrypted = encrypted.and_then(|enc| match common_crypto::decrypt(enc) {
        Ok(p) => Some(p),
        Err(e) => {
            tracing::warn!(error = %e, "gagal dekripsi phone_encrypted — fallback ke plaintext lama");
            None
        }
    });
    decrypted.or(plaintext_fallback)
}

#[allow(dead_code)]
/// Decrypt rekening JSON. Gagal → warn + None.
fn decrypt_rekening(encrypted: Option<&str>) -> Option<RekeningInfo> {
    encrypted.and_then(|enc| match common_crypto::decrypt(enc) {
        Ok(json) => match serde_json::from_str(&json) {
            Ok(r) => Some(r),
            Err(e) => {
                tracing::warn!(error = %e, "gagal parse JSON rekening setelah dekripsi");
                None
            }
        },
        Err(e) => {
            tracing::warn!(error = %e, "gagal dekripsi rekening_encrypted");
            None
        }
    })
}

/// Map satu PgRow (semua kolom `user_svc.profiles`) ke `UserProfile`,
/// termasuk dekripsi phone_encrypted + rekening_encrypted.
fn row_to_profile(r: &PgRow) -> UserProfile {
    let phone_encrypted: Option<String> = r.get("phone_encrypted");
    let rekening_encrypted: Option<String> = r.get("rekening_encrypted");
    let phone_plain: Option<String> = r.get("phone");

    UserProfile {
        id: r.get("id"),
        auth_id: r.get("auth_id"),
        username: r.get("username"),
        full_name: r.get("full_name"),
        avatar: r.get("avatar"),
        bio: r.get("bio"),
        phone: decrypt_phone(phone_encrypted.as_deref(), phone_plain),
        phone_encrypted,
        rekening_encrypted,
        nik_encrypted: r.get("nik_encrypted"),
        nik_last4: r.get("nik_last4"),
        education_level: r.get("education_level"),
        gender: r.get("gender"),
        birth_date: r.get("birth_date"),
        address_line: r.get("address_line"),
        country_code: r.get("country_code"),
        province_id: r.get("province_id"),
        regency_id: r.get("regency_id"),
        district_id: r.get("district_id"),
        village_id: r.get("village_id"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    }
}

// SQL listing/detail admin KYC — literal statis (sqlx 0.9 menolak dynamic string;
// literal = anti SQL-injection). Kolom konsisten antar-query; hanya klausa
// search/order/limit yang berbeda per-cabang.
//
// Daftar kolom dasar (submission ⋈ profile), dipakai di semua varian:
//   k.id AS submission_id, k.status, k.created_at, k.ktp_object_key,
//   k.selfie_object_key, p.id AS profile_id, p.full_name, p.nik_last4,
//   p.education_level, p.gender, p.birth_date, p.address_line, p.country_code,
//   p.province_id, p.regency_id, p.district_id, p.village_id
const ADMIN_KYC_LIST_SEARCH_ASC: &str = "SELECT k.id AS submission_id, k.status, k.created_at, k.ktp_object_key, k.selfie_object_key, p.id AS profile_id, p.full_name, p.nik_last4, p.education_level, p.gender, p.birth_date, p.address_line, p.country_code, p.province_id, p.regency_id, p.district_id, p.village_id, COUNT(*) OVER() AS total_rows FROM user_svc.kyc_submission k JOIN user_svc.profiles p ON p.id = k.profile_id WHERE k.status = $1 AND (p.full_name ILIKE $2 OR k.id::text ILIKE $2 OR p.id::text ILIKE $2) ORDER BY k.created_at ASC LIMIT $3 OFFSET $4";
const ADMIN_KYC_LIST_SEARCH_DESC: &str = "SELECT k.id AS submission_id, k.status, k.created_at, k.ktp_object_key, k.selfie_object_key, p.id AS profile_id, p.full_name, p.nik_last4, p.education_level, p.gender, p.birth_date, p.address_line, p.country_code, p.province_id, p.regency_id, p.district_id, p.village_id, COUNT(*) OVER() AS total_rows FROM user_svc.kyc_submission k JOIN user_svc.profiles p ON p.id = k.profile_id WHERE k.status = $1 AND (p.full_name ILIKE $2 OR k.id::text ILIKE $2 OR p.id::text ILIKE $2) ORDER BY k.created_at DESC LIMIT $3 OFFSET $4";
const ADMIN_KYC_LIST_ASC: &str = "SELECT k.id AS submission_id, k.status, k.created_at, k.ktp_object_key, k.selfie_object_key, p.id AS profile_id, p.full_name, p.nik_last4, p.education_level, p.gender, p.birth_date, p.address_line, p.country_code, p.province_id, p.regency_id, p.district_id, p.village_id, COUNT(*) OVER() AS total_rows FROM user_svc.kyc_submission k JOIN user_svc.profiles p ON p.id = k.profile_id WHERE k.status = $1 ORDER BY k.created_at ASC LIMIT $2 OFFSET $3";
const ADMIN_KYC_LIST_DESC: &str = "SELECT k.id AS submission_id, k.status, k.created_at, k.ktp_object_key, k.selfie_object_key, p.id AS profile_id, p.full_name, p.nik_last4, p.education_level, p.gender, p.birth_date, p.address_line, p.country_code, p.province_id, p.regency_id, p.district_id, p.village_id, COUNT(*) OVER() AS total_rows FROM user_svc.kyc_submission k JOIN user_svc.profiles p ON p.id = k.profile_id WHERE k.status = $1 ORDER BY k.created_at DESC LIMIT $2 OFFSET $3";
const ADMIN_KYC_EXPORT_SEARCH: &str = "SELECT k.id AS submission_id, k.status, k.created_at, k.ktp_object_key, k.selfie_object_key, p.id AS profile_id, p.full_name, p.nik_last4, p.education_level, p.gender, p.birth_date, p.address_line, p.country_code, p.province_id, p.regency_id, p.district_id, p.village_id FROM user_svc.kyc_submission k JOIN user_svc.profiles p ON p.id = k.profile_id WHERE k.status = $1 AND (p.full_name ILIKE $2 OR k.id::text ILIKE $2 OR p.id::text ILIKE $2) ORDER BY k.created_at DESC LIMIT $3";
const ADMIN_KYC_EXPORT: &str = "SELECT k.id AS submission_id, k.status, k.created_at, k.ktp_object_key, k.selfie_object_key, p.id AS profile_id, p.full_name, p.nik_last4, p.education_level, p.gender, p.birth_date, p.address_line, p.country_code, p.province_id, p.regency_id, p.district_id, p.village_id FROM user_svc.kyc_submission k JOIN user_svc.profiles p ON p.id = k.profile_id WHERE k.status = $1 ORDER BY k.created_at DESC LIMIT $2";
const ADMIN_KYC_DETAIL: &str = "SELECT k.id AS submission_id, k.status, k.created_at, k.ktp_object_key, k.selfie_object_key, p.id AS profile_id, p.full_name, p.nik_last4, p.education_level, p.gender, p.birth_date, p.address_line, p.country_code, p.province_id, p.regency_id, p.district_id, p.village_id FROM user_svc.kyc_submission k JOIN user_svc.profiles p ON p.id = k.profile_id WHERE k.id = $1";

/// Map PgRow → AdminKycRow. Dipakai oleh listing & detail (DRY, hindari duplikasi).
/// Kolom nullable di-bind ke `Option<_>` eksplisit agar NULL → None (bukan Err).
fn row_to_admin_kyc_row(r: &PgRow) -> AdminKycRow {
    let status_raw: String = r.get("status");
    let status = match status_raw.parse() {
        Ok(s) => s,
        Err(()) => {
            tracing::warn!(
                status = %status_raw,
                submission_id = %r.get::<uuid::Uuid, _>("submission_id"),
                "status KYC tidak dikenal di DB — fallback ke Pending"
            );
            KycSubmissionStatus::Pending
        }
    };
    AdminKycRow {
        submission_id: r.get("submission_id"),
        status,
        created_at: r.get("created_at"),
        profile_id: r.get("profile_id"),
        full_name: r.get("full_name"),
        nik_last4: r.get("nik_last4"),
        education_level: r.get("education_level"),
        gender: r.get("gender"),
        birth_date: r.get("birth_date"),
        address_line: r.get("address_line"),
        country_code: r.get("country_code"),
        province_id: r.get("province_id"),
        regency_id: r.get("regency_id"),
        district_id: r.get("district_id"),
        village_id: r.get("village_id"),
        ktp_object_key: r.get("ktp_object_key"),
        selfie_object_key: r.get("selfie_object_key"),
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

pub struct PgUserRepository {
    pool: PgPool,
}

impl PgUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl UserRepository for PgUserRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<UserProfile>, anyhow::Error> {
        let t = Instant::now();
        let row = sqlx::query(
            "SELECT id, auth_id, username, full_name, avatar, bio, phone, \
                    phone_encrypted, rekening_encrypted, \
                    nik_encrypted, nik_last4, education_level, gender, birth_date, \
                    address_line, country_code, province_id, regency_id, district_id, village_id, \
                    created_at, updated_at \
             FROM user_svc.profiles WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        warn_slow!(t, "user.find_by_id");
        Ok(row.as_ref().map(row_to_profile))
    }

    async fn find_by_auth_id(&self, auth_id: Uuid) -> Result<Option<UserProfile>, anyhow::Error> {
        let t = Instant::now();
        let row = sqlx::query(
            "SELECT id, auth_id, username, full_name, avatar, bio, phone, \
                    phone_encrypted, rekening_encrypted, \
                    nik_encrypted, nik_last4, education_level, gender, birth_date, \
                    address_line, country_code, province_id, regency_id, district_id, village_id, \
                    created_at, updated_at \
             FROM user_svc.profiles WHERE auth_id = $1",
        )
        .bind(auth_id)
        .fetch_optional(&self.pool)
        .await?;
        warn_slow!(t, "user.find_by_auth_id");
        Ok(row.as_ref().map(row_to_profile))
    }

    async fn create(&self, auth_id: Uuid, username: &str) -> Result<UserProfile, anyhow::Error> {
        let t = Instant::now();
        let row = sqlx::query(
            "INSERT INTO user_svc.profiles (id, auth_id, username) \
             VALUES (gen_random_uuid(), $1, $2) \
             RETURNING id, auth_id, username, full_name, avatar, bio, phone, \
                       phone_encrypted, rekening_encrypted, \
                       nik_encrypted, nik_last4, education_level, gender, birth_date, \
                       address_line, country_code, province_id, regency_id, district_id, village_id, \
                       created_at, updated_at",
        )
        .bind(auth_id)
        .bind(username)
        .fetch_one(&self.pool)
        .await?;
        warn_slow!(t, "user.create");
        Ok(row_to_profile(&row))
    }

    async fn update(&self, params: UpdateProfileParams) -> Result<UserProfile, anyhow::Error> {
        let t = Instant::now();

        // Encrypt phone bila ada.
        let phone_encrypted = params
            .phone
            .as_deref()
            .map(common_crypto::encrypt)
            .transpose()
            .map_err(|e| anyhow::anyhow!("gagal enkripsi phone: {e}"))?;

        // Encrypt rekening bila ada.
        let rekening_encrypted = params
            .rekening
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| anyhow::anyhow!("gagal serialise rekening: {e}"))?
            .map(|json| common_crypto::encrypt(&json))
            .transpose()
            .map_err(|e| anyhow::anyhow!("gagal enkripsi rekening: {e}"))?;

        let row = sqlx::query(
            "UPDATE user_svc.profiles SET \
                full_name          = COALESCE($2, full_name), \
                avatar             = COALESCE($3, avatar), \
                bio                = COALESCE($4, bio), \
                phone_encrypted    = COALESCE($5, phone_encrypted), \
                rekening_encrypted = COALESCE($6, rekening_encrypted), \
                updated_at         = now() \
             WHERE id = $1 \
             RETURNING id, auth_id, username, full_name, avatar, bio, phone, \
                       phone_encrypted, rekening_encrypted, \
                       nik_encrypted, nik_last4, education_level, gender, birth_date, \
                       address_line, country_code, province_id, regency_id, district_id, village_id, \
                       created_at, updated_at",
        )
        .bind(params.id)
        .bind(&params.full_name)
        .bind(&params.avatar)
        .bind(&params.bio)
        .bind(&phone_encrypted)
        .bind(&rekening_encrypted)
        .fetch_one(&self.pool)
        .await?;
        warn_slow!(t, "user.update");
        Ok(row_to_profile(&row))
    }

    async fn update_avatar(&self, id: Uuid, object_key: &str) -> Result<(), anyhow::Error> {
        let t = Instant::now();
        sqlx::query!(
            "UPDATE user_svc.profiles SET avatar = $2, updated_at = now() WHERE id = $1",
            id,
            object_key
        )
        .execute(&self.pool)
        .await?;
        warn_slow!(t, "user.update_avatar");
        Ok(())
    }

    /// Atomic update profile — guard `nik_encrypted IS NULL` mencegah overwrite
    /// concurrent (C1). Return `true` bila baris terupdate, `false` bila NIK sudah ada.
    async fn update_profile(&self, profile: &UserProfile) -> Result<bool, anyhow::Error> {
        let t = Instant::now();
        // Pakai sqlx::query (bukan query!) agar kompatibel dengan SQLX_OFFLINE;
        // query sama dengan sebelumnya, hanya ditambah guard `AND nik_encrypted IS NULL`.
        let result = sqlx::query(
            "UPDATE user_svc.profiles SET
                full_name = $2, nik_encrypted = $3, nik_last4 = $4,
                education_level = $5, gender = $6, birth_date = $7,
                address_line = $8, country_code = $9,
                province_id = $10, regency_id = $11, district_id = $12, village_id = $13,
                updated_at = now()
             WHERE id = $1 AND nik_encrypted IS NULL",
        )
        .bind(profile.id)
        .bind(&profile.full_name)
        .bind(&profile.nik_encrypted)
        .bind(&profile.nik_last4)
        .bind(&profile.education_level)
        .bind(&profile.gender)
        .bind(profile.birth_date)
        .bind(&profile.address_line)
        .bind(&profile.country_code)
        .bind(&profile.province_id)
        .bind(&profile.regency_id)
        .bind(&profile.district_id)
        .bind(&profile.village_id)
        .execute(&self.pool)
        .await?;
        warn_slow!(t, "user.update_profile");
        Ok(result.rows_affected() > 0)
    }

    // ── KYC submission ─────────────────────────────────────────────────────────

    async fn create_submission(&self, profile_id: Uuid) -> Result<KycSubmission, anyhow::Error> {
        let t = Instant::now();
        let row = sqlx::query!(
            "INSERT INTO user_svc.kyc_submission (id, profile_id, status)
             VALUES (gen_random_uuid(), $1, 'pending')
             RETURNING id, profile_id, status, created_at, updated_at",
            profile_id
        )
        .fetch_one(&self.pool)
        .await?;
        warn_slow!(t, "kyc.create_submission");
        Ok(KycSubmission {
            id: row.id,
            profile_id: row.profile_id,
            status: KycSubmissionStatus::Pending,
            ktp_object_key: None,
            selfie_object_key: None,
            reviewed_by: None,
            review_note: None,
            reviewed_at: None,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    async fn get_latest_submission(
        &self,
        profile_id: Uuid,
    ) -> Result<Option<KycSubmission>, anyhow::Error> {
        let t = Instant::now();
        let row = sqlx::query!(
            "SELECT id, profile_id, status, ktp_object_key, selfie_object_key,
                    reviewed_by, review_note, reviewed_at, created_at, updated_at
             FROM user_svc.kyc_submission
             WHERE profile_id = $1
             ORDER BY created_at DESC LIMIT 1",
            profile_id
        )
        .fetch_optional(&self.pool)
        .await?;
        warn_slow!(t, "kyc.get_latest_submission");
        row.map(|r| {
            Ok(KycSubmission {
                id: r.id,
                profile_id: r.profile_id,
                status: r.status.parse().unwrap_or(KycSubmissionStatus::Pending),
                ktp_object_key: r.ktp_object_key,
                selfie_object_key: r.selfie_object_key,
                reviewed_by: r.reviewed_by,
                review_note: r.review_note,
                reviewed_at: r.reviewed_at,
                created_at: r.created_at,
                updated_at: r.updated_at,
            })
        })
        .transpose()
    }

    async fn get_submission_by_id(
        &self,
        submission_id: Uuid,
    ) -> Result<Option<KycSubmission>, anyhow::Error> {
        let t = Instant::now();
        let row = sqlx::query!(
            "SELECT id, profile_id, status, ktp_object_key, selfie_object_key,
                    reviewed_by, review_note, reviewed_at, created_at, updated_at
             FROM user_svc.kyc_submission
             WHERE id = $1",
            submission_id
        )
        .fetch_optional(&self.pool)
        .await?;
        warn_slow!(t, "kyc.get_submission_by_id");
        row.map(|r| {
            Ok(KycSubmission {
                id: r.id,
                profile_id: r.profile_id,
                status: r.status.parse().unwrap_or(KycSubmissionStatus::Pending),
                ktp_object_key: r.ktp_object_key,
                selfie_object_key: r.selfie_object_key,
                reviewed_by: r.reviewed_by,
                review_note: r.review_note,
                reviewed_at: r.reviewed_at,
                created_at: r.created_at,
                updated_at: r.updated_at,
            })
        })
        .transpose()
    }

    /// Atomic review: UPDATE hanya bila `status = 'pending'` — eliminasi TOCTOU
    /// race condition (add-user-admin-management code review). Mengembalikan true
    /// bila baris berhasil diupdate (submission berhasil ditinjau), false bila
    /// submission sudah terminal (pemanggil harus menolak dengan 409/AlreadyReviewed).
    async fn review_submission(
        &self,
        id: Uuid,
        status: KycSubmissionStatus,
        reviewed_by: Uuid,
        review_note: Option<&str>,
    ) -> Result<bool, anyhow::Error> {
        let t = Instant::now();
        let result = sqlx::query(
            "UPDATE user_svc.kyc_submission
             SET status = $2, reviewed_by = $3, review_note = $4, reviewed_at = now(), updated_at = now()
             WHERE id = $1 AND status = 'pending'",
        )
        .bind(id)
        .bind(status.as_str())
        .bind(reviewed_by)
        .bind(review_note)
        .execute(&self.pool)
        .await?;
        warn_slow!(t, "kyc.review_submission");
        Ok(result.rows_affected() > 0)
    }

    async fn set_document_key(
        &self,
        submission_id: Uuid,
        kind: &str,
        object_key: &str,
    ) -> Result<(), anyhow::Error> {
        let t = Instant::now();
        match kind {
            "ktp" => {
                sqlx::query(
                    "UPDATE user_svc.kyc_submission SET ktp_object_key = $2, updated_at = now() WHERE id = $1",
                )
                .bind(submission_id)
                .bind(object_key)
                .execute(&self.pool)
                .await?;
            }
            "selfie" => {
                sqlx::query(
                    "UPDATE user_svc.kyc_submission SET selfie_object_key = $2, updated_at = now() WHERE id = $1",
                )
                .bind(submission_id)
                .bind(object_key)
                .execute(&self.pool)
                .await?;
            }
            _ => return Err(anyhow::anyhow!("jenis dokumen tidak dikenal: {kind}")),
        }
        warn_slow!(t, "kyc.set_document_key");
        Ok(())
    }

    async fn clear_document_keys(&self, submission_id: Uuid) -> Result<(), anyhow::Error> {
        let t = Instant::now();
        sqlx::query(
            "UPDATE user_svc.kyc_submission SET ktp_object_key = NULL, selfie_object_key = NULL, updated_at = now() WHERE id = $1",
        )
        .bind(submission_id)
        .execute(&self.pool)
        .await?;
        warn_slow!(t, "kyc.clear_document_keys");
        Ok(())
    }

    async fn log_document_access(
        &self,
        actor_id: Uuid,
        object_key: &str,
        action: DocumentAccessAction,
        request_id: Option<&str>,
    ) -> Result<(), anyhow::Error> {
        let t = Instant::now();
        sqlx::query(
            "INSERT INTO user_svc.document_access_log (actor_id, object_key, action, request_id) VALUES ($1, $2, $3, $4)",
        )
        .bind(actor_id)
        .bind(object_key)
        .bind(action.as_str())
        .bind(request_id)
        .execute(&self.pool)
        .await?;
        warn_slow!(t, "kyc.log_document_access");
        Ok(())
    }

    // ── Admin listing KYC (add-user-admin-management) ───────────────────────────

    async fn admin_list_submissions(
        &self,
        params: AdminKycListParams,
    ) -> Result<AdminKycListResult, anyhow::Error> {
        let t = Instant::now();
        let status = params.status.as_deref().unwrap_or("pending");
        let q_pattern = params
            .q
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| format!("%{s}%"));
        let asc = matches!(params.sort_dir.as_deref(), Some("asc"));

        // COUNT(*) OVER() — total dalam satu round-trip (Zero N+1).
        // SQL berupa literal statis per-cabang (bukan format!) — sqlx 0.9 menolak
        // dynamic string; ini sekaligus jaminan anti SQL-injection.
        let rows = match (&q_pattern, asc) {
            (Some(q), true) => {
                sqlx::query(ADMIN_KYC_LIST_SEARCH_ASC)
                    .bind(status)
                    .bind(q)
                    .bind(params.limit)
                    .bind(params.offset)
                    .fetch_all(&self.pool)
                    .await?
            }
            (Some(q), false) => {
                sqlx::query(ADMIN_KYC_LIST_SEARCH_DESC)
                    .bind(status)
                    .bind(q)
                    .bind(params.limit)
                    .bind(params.offset)
                    .fetch_all(&self.pool)
                    .await?
            }
            (None, true) => {
                sqlx::query(ADMIN_KYC_LIST_ASC)
                    .bind(status)
                    .bind(params.limit)
                    .bind(params.offset)
                    .fetch_all(&self.pool)
                    .await?
            }
            (None, false) => {
                sqlx::query(ADMIN_KYC_LIST_DESC)
                    .bind(status)
                    .bind(params.limit)
                    .bind(params.offset)
                    .fetch_all(&self.pool)
                    .await?
            }
        };
        warn_slow!(t, "kyc.admin_list_submissions");

        let total: i64 = rows
            .first()
            .map_or(0, |r| r.try_get::<i64, _>("total_rows").unwrap_or(0));
        Ok(AdminKycListResult {
            items: rows.iter().map(row_to_admin_kyc_row).collect(),
            total,
        })
    }

    async fn admin_list_submissions_all(
        &self,
        params: AdminKycListParams,
    ) -> Result<Vec<AdminKycRow>, anyhow::Error> {
        let t = Instant::now();
        let status = params.status.as_deref().unwrap_or("pending");
        let q_pattern = params
            .q
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| format!("%{s}%"));

        let rows = match &q_pattern {
            Some(q) => {
                sqlx::query(ADMIN_KYC_EXPORT_SEARCH)
                    .bind(status)
                    .bind(q)
                    .bind(params.limit)
                    .fetch_all(&self.pool)
                    .await?
            }
            None => {
                sqlx::query(ADMIN_KYC_EXPORT)
                    .bind(status)
                    .bind(params.limit)
                    .fetch_all(&self.pool)
                    .await?
            }
        };
        warn_slow!(t, "kyc.admin_list_submissions_all");
        Ok(rows.iter().map(row_to_admin_kyc_row).collect())
    }

    async fn get_submission_with_profile(
        &self,
        submission_id: Uuid,
    ) -> Result<Option<AdminKycRow>, anyhow::Error> {
        let t = Instant::now();
        let row = sqlx::query(ADMIN_KYC_DETAIL)
            .bind(submission_id)
            .fetch_optional(&self.pool)
            .await?;
        warn_slow!(t, "kyc.get_submission_with_profile");
        Ok(row.as_ref().map(row_to_admin_kyc_row))
    }

    async fn begin_transaction(&self) -> Result<Box<dyn TxUserRepository>, anyhow::Error> {
        let tx = self.pool.begin().await?;
        Ok(Box::new(PgTxUserRepository { tx }))
    }
}

/// Transaction wrapper — implementasi `TxUserRepository` di atas PostgreSQL transaction.
/// Dipakai oleh `submit_kyc` untuk atomic multi-step write (C2).
struct PgTxUserRepository {
    tx: sqlx::Transaction<'static, sqlx::Postgres>,
}

#[async_trait::async_trait]
impl TxUserRepository for PgTxUserRepository {
    async fn find_by_auth_id(
        &mut self,
        auth_id: Uuid,
    ) -> Result<Option<UserProfile>, anyhow::Error> {
        let row = sqlx::query(
            "SELECT id, auth_id, username, full_name, avatar, bio, phone, \
                    phone_encrypted, rekening_encrypted, \
                    nik_encrypted, nik_last4, education_level, gender, birth_date, \
                    address_line, country_code, province_id, regency_id, district_id, village_id, \
                    created_at, updated_at \
             FROM user_svc.profiles WHERE auth_id = $1",
        )
        .bind(auth_id)
        .fetch_optional(&mut *self.tx)
        .await?;
        Ok(row.as_ref().map(row_to_profile))
    }

    async fn update_profile(&mut self, profile: &UserProfile) -> Result<bool, anyhow::Error> {
        let result = sqlx::query(
            "UPDATE user_svc.profiles SET
                full_name = $2, nik_encrypted = $3, nik_last4 = $4,
                education_level = $5, gender = $6, birth_date = $7,
                address_line = $8, country_code = $9,
                province_id = $10, regency_id = $11, district_id = $12, village_id = $13,
                updated_at = now()
             WHERE id = $1 AND nik_encrypted IS NULL",
        )
        .bind(profile.id)
        .bind(&profile.full_name)
        .bind(&profile.nik_encrypted)
        .bind(&profile.nik_last4)
        .bind(&profile.education_level)
        .bind(&profile.gender)
        .bind(profile.birth_date)
        .bind(&profile.address_line)
        .bind(&profile.country_code)
        .bind(&profile.province_id)
        .bind(&profile.regency_id)
        .bind(&profile.district_id)
        .bind(&profile.village_id)
        .execute(&mut *self.tx)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn create_submission(
        &mut self,
        profile_id: Uuid,
    ) -> Result<KycSubmission, anyhow::Error> {
        let row = sqlx::query!(
            "INSERT INTO user_svc.kyc_submission (id, profile_id, status)
             VALUES (gen_random_uuid(), $1, 'pending')
             RETURNING id, profile_id, status, created_at, updated_at",
            profile_id
        )
        .fetch_one(&mut *self.tx)
        .await?;
        Ok(KycSubmission {
            id: row.id,
            profile_id: row.profile_id,
            status: KycSubmissionStatus::Pending,
            ktp_object_key: None,
            selfie_object_key: None,
            reviewed_by: None,
            review_note: None,
            reviewed_at: None,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    async fn commit(self: Box<Self>) -> Result<(), anyhow::Error> {
        self.tx.commit().await?;
        Ok(())
    }

    async fn rollback(self: Box<Self>) -> Result<(), anyhow::Error> {
        self.tx.rollback().await?;
        Ok(())
    }
}
