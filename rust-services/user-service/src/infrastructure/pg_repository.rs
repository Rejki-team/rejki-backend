use std::time::Instant;

use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::entity::{KycSubmission, KycSubmissionStatus, UserProfile};
use crate::domain::repository::UserRepository;

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

#[async_trait::async_trait]
impl UserRepository for PgUserRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<UserProfile>, anyhow::Error> {
        let t = Instant::now();
        let row = sqlx::query!(
            r#"SELECT id, auth_id, username, full_name, avatar, bio, phone,
                      nik_encrypted, nik_last4, education_level, gender, birth_date,
                      address_line, country_code, province_id, regency_id, district_id, village_id,
                      created_at, updated_at
               FROM user_svc.profiles WHERE id = $1"#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;
        warn_slow!(t, "user.find_by_id");
        Ok(row.map(|r| UserProfile {
            id:              r.id,
            auth_id:         r.auth_id,
            username:        r.username,
            full_name:       r.full_name,
            avatar:          r.avatar,
            bio:             r.bio,
            phone:           r.phone,
            nik_encrypted:   r.nik_encrypted,
            nik_last4:       r.nik_last4,
            education_level: r.education_level,
            gender:          r.gender,
            birth_date:      r.birth_date,
            address_line:    r.address_line,
            country_code:    r.country_code,
            province_id:     r.province_id,
            regency_id:      r.regency_id,
            district_id:     r.district_id,
            village_id:      r.village_id,
            created_at:      r.created_at,
            updated_at:      r.updated_at,
        }))
    }

    async fn find_by_auth_id(&self, auth_id: Uuid) -> Result<Option<UserProfile>, anyhow::Error> {
        let t = Instant::now();
        let row = sqlx::query!(
            r#"SELECT id, auth_id, username, full_name, avatar, bio, phone,
                      nik_encrypted, nik_last4, education_level, gender, birth_date,
                      address_line, country_code, province_id, regency_id, district_id, village_id,
                      created_at, updated_at
               FROM user_svc.profiles WHERE auth_id = $1"#,
            auth_id
        )
        .fetch_optional(&self.pool)
        .await?;
        warn_slow!(t, "user.find_by_auth_id");
        Ok(row.map(|r| UserProfile {
            id:              r.id,
            auth_id:         r.auth_id,
            username:        r.username,
            full_name:       r.full_name,
            avatar:          r.avatar,
            bio:             r.bio,
            phone:           r.phone,
            nik_encrypted:   r.nik_encrypted,
            nik_last4:       r.nik_last4,
            education_level: r.education_level,
            gender:          r.gender,
            birth_date:      r.birth_date,
            address_line:    r.address_line,
            country_code:    r.country_code,
            province_id:     r.province_id,
            regency_id:      r.regency_id,
            district_id:     r.district_id,
            village_id:      r.village_id,
            created_at:      r.created_at,
            updated_at:      r.updated_at,
        }))
    }

    async fn create(&self, auth_id: Uuid, username: &str) -> Result<UserProfile, anyhow::Error> {
        let t = Instant::now();
        let row = sqlx::query!(
            r#"INSERT INTO user_svc.profiles (id, auth_id, username)
               VALUES (gen_random_uuid(), $1, $2)
               RETURNING id, auth_id, username, full_name, avatar, bio, phone,
                         nik_encrypted, nik_last4, education_level, gender, birth_date,
                         address_line, country_code, province_id, regency_id, district_id, village_id,
                         created_at, updated_at"#,
            auth_id, username
        )
        .fetch_one(&self.pool)
        .await?;
        warn_slow!(t, "user.create");
        Ok(UserProfile {
            id:              row.id,
            auth_id:         row.auth_id,
            username:        row.username,
            full_name:       row.full_name,
            avatar:          row.avatar,
            bio:             row.bio,
            phone:           row.phone,
            nik_encrypted:   row.nik_encrypted,
            nik_last4:       row.nik_last4,
            education_level: row.education_level,
            gender:          row.gender,
            birth_date:      row.birth_date,
            address_line:    row.address_line,
            country_code:    row.country_code,
            province_id:     row.province_id,
            regency_id:      row.regency_id,
            district_id:     row.district_id,
            village_id:      row.village_id,
            created_at:      row.created_at,
            updated_at:      row.updated_at,
        })
    }

    async fn update(
        &self,
        id: Uuid,
        full_name: Option<&str>,
        avatar: Option<&str>,
        bio: Option<&str>,
        phone: Option<&str>,
    ) -> Result<UserProfile, anyhow::Error> {
        let t = Instant::now();
        let row = sqlx::query!(
            r#"UPDATE user_svc.profiles
               SET full_name  = COALESCE($2, full_name),
                   avatar     = COALESCE($3, avatar),
                   bio        = COALESCE($4, bio),
                   phone      = COALESCE($5, phone),
                   updated_at = now()
               WHERE id = $1
               RETURNING id, auth_id, username, full_name, avatar, bio, phone,
                         nik_encrypted, nik_last4, education_level, gender, birth_date,
                         address_line, country_code, province_id, regency_id, district_id, village_id,
                         created_at, updated_at"#,
            id, full_name, avatar, bio, phone
        )
        .fetch_one(&self.pool)
        .await?;
        warn_slow!(t, "user.update");
        Ok(UserProfile {
            id:              row.id,
            auth_id:         row.auth_id,
            username:        row.username,
            full_name:       row.full_name,
            avatar:          row.avatar,
            bio:             row.bio,
            phone:           row.phone,
            nik_encrypted:   row.nik_encrypted,
            nik_last4:       row.nik_last4,
            education_level: row.education_level,
            gender:          row.gender,
            birth_date:      row.birth_date,
            address_line:    row.address_line,
            country_code:    row.country_code,
            province_id:     row.province_id,
            regency_id:      row.regency_id,
            district_id:     row.district_id,
            village_id:      row.village_id,
            created_at:      row.created_at,
            updated_at:      row.updated_at,
        })
    }

    async fn update_avatar(&self, id: Uuid, object_key: &str) -> Result<(), anyhow::Error> {
        sqlx::query!(
            "UPDATE user_svc.profiles SET avatar = $2, updated_at = now() WHERE id = $1",
            id, object_key
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn update_profile(&self, profile: &UserProfile) -> Result<(), anyhow::Error> {
        sqlx::query!(
            "UPDATE user_svc.profiles SET
                full_name = $2, nik_encrypted = $3, nik_last4 = $4,
                education_level = $5, gender = $6, birth_date = $7,
                address_line = $8, country_code = $9,
                province_id = $10, regency_id = $11, district_id = $12, village_id = $13,
                updated_at = now()
             WHERE id = $1",
            profile.id,
            profile.full_name,
            profile.nik_encrypted,
            profile.nik_last4,
            profile.education_level,
            profile.gender,
            profile.birth_date,
            profile.address_line,
            profile.country_code,
            profile.province_id,
            profile.regency_id,
            profile.district_id,
            profile.village_id,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // ── KYC submission ─────────────────────────────────────────────────────────

    async fn create_submission(&self, profile_id: Uuid) -> Result<KycSubmission, anyhow::Error> {
        let row = sqlx::query!(
            "INSERT INTO user_svc.kyc_submission (id, profile_id, status)
             VALUES (gen_random_uuid(), $1, 'pending')
             RETURNING id, profile_id, status, created_at, updated_at",
            profile_id
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(KycSubmission {
            id:                row.id,
            profile_id:        row.profile_id,
            status:            KycSubmissionStatus::Pending,
            ktp_object_key:    None,
            selfie_object_key: None,
            reviewed_by:       None,
            review_note:       None,
            reviewed_at:       None,
            created_at:        row.created_at,
            updated_at:        row.updated_at,
        })
    }

    async fn get_latest_submission(&self, profile_id: Uuid) -> Result<Option<KycSubmission>, anyhow::Error> {
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
        row.map(|r| {
            Ok(KycSubmission {
                id:                r.id,
                profile_id:        r.profile_id,
                status:            r.status.parse().unwrap_or(KycSubmissionStatus::Pending),
                ktp_object_key:    r.ktp_object_key,
                selfie_object_key: r.selfie_object_key,
                reviewed_by:       r.reviewed_by,
                review_note:       r.review_note,
                reviewed_at:       r.reviewed_at,
                created_at:        r.created_at,
                updated_at:        r.updated_at,
            })
        })
        .transpose()
    }

    async fn get_submission_by_id(&self, submission_id: Uuid) -> Result<Option<KycSubmission>, anyhow::Error> {
        let row = sqlx::query!(
            "SELECT id, profile_id, status, ktp_object_key, selfie_object_key,
                    reviewed_by, review_note, reviewed_at, created_at, updated_at
             FROM user_svc.kyc_submission
             WHERE id = $1",
            submission_id
        )
        .fetch_optional(&self.pool)
        .await?;
        row.map(|r| {
            Ok(KycSubmission {
                id:                r.id,
                profile_id:        r.profile_id,
                status:            r.status.parse().unwrap_or(KycSubmissionStatus::Pending),
                ktp_object_key:    r.ktp_object_key,
                selfie_object_key: r.selfie_object_key,
                reviewed_by:       r.reviewed_by,
                review_note:       r.review_note,
                reviewed_at:       r.reviewed_at,
                created_at:        r.created_at,
                updated_at:        r.updated_at,
            })
        })
        .transpose()
    }

    async fn review_submission(
        &self,
        id:          Uuid,
        status:      KycSubmissionStatus,
        reviewed_by: Uuid,
        review_note: Option<&str>,
    ) -> Result<(), anyhow::Error> {
        sqlx::query!(
            "UPDATE user_svc.kyc_submission
             SET status = $2, reviewed_by = $3, review_note = $4, reviewed_at = now(), updated_at = now()
             WHERE id = $1",
            id, status.as_str(), reviewed_by, review_note
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
