use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::entity::{AccountStatus, AuthUser};
use crate::domain::repository::AuthRepository;

pub struct PgAuthRepository {
    pool: PgPool,
}

impl PgAuthRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Map string status dari DB ke enum domain; status tak dikenal dianggap error data.
fn parse_status(s: &str) -> Result<AccountStatus, anyhow::Error> {
    s.parse()
        .map_err(|_| anyhow::anyhow!("status akun tidak dikenal: {s}"))
}

impl AuthRepository for PgAuthRepository {
    async fn find_by_email(&self, email: &str) -> Result<Option<AuthUser>, anyhow::Error> {
        let row = sqlx::query!(
            "SELECT id, email, password_hash, status, phone, tos_accepted_at, tos_version, created_at, updated_at
             FROM auth.users WHERE email = $1",
            email
        )
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| -> Result<AuthUser, anyhow::Error> {
            Ok(AuthUser {
                id: r.id,
                email: r.email,
                password_hash: r.password_hash,
                status: parse_status(&r.status)?,
                phone: r.phone,
                tos_accepted_at: r.tos_accepted_at,
                tos_version: r.tos_version,
                created_at: r.created_at,
                updated_at: r.updated_at,
            })
        })
        .transpose()
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<AuthUser>, anyhow::Error> {
        let row = sqlx::query!(
            "SELECT id, email, password_hash, status, phone, tos_accepted_at, tos_version, created_at, updated_at
             FROM auth.users WHERE id = $1",
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| -> Result<AuthUser, anyhow::Error> {
            Ok(AuthUser {
                id: r.id,
                email: r.email,
                password_hash: r.password_hash,
                status: parse_status(&r.status)?,
                phone: r.phone,
                tos_accepted_at: r.tos_accepted_at,
                tos_version: r.tos_version,
                created_at: r.created_at,
                updated_at: r.updated_at,
            })
        })
        .transpose()
    }

    async fn create_user(
        &self,
        email: &str,
        password_hash: &str,
        phone_encrypted: Option<&str>,
        tos_version: &str,
    ) -> Result<AuthUser, anyhow::Error> {
        let row = sqlx::query!(
            "INSERT INTO auth.users (id, email, password_hash, phone, tos_accepted_at, tos_version)
             VALUES (gen_random_uuid(), $1, $2, $3, now(), $4)
             RETURNING id, email, password_hash, status, phone, tos_accepted_at, tos_version, created_at, updated_at",
            email,
            password_hash,
            phone_encrypted,
            tos_version
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(AuthUser {
            id: row.id,
            email: row.email,
            password_hash: row.password_hash,
            status: parse_status(&row.status)?,
            phone: row.phone,
            tos_accepted_at: row.tos_accepted_at,
            tos_version: row.tos_version,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    async fn user_exists(&self, user_id: Uuid) -> Result<bool, anyhow::Error> {
        Ok(sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM auth.users WHERE id = $1)",
            user_id
        )
        .fetch_one(&self.pool)
        .await?
        .unwrap_or(false))
    }

    async fn save_refresh_token(
        &self,
        user_id: Uuid,
        token_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), anyhow::Error> {
        sqlx::query!(
            "INSERT INTO auth.refresh_tokens (id, user_id, token_hash, expires_at)
             VALUES (gen_random_uuid(), $1, $2, $3)",
            user_id,
            token_hash,
            expires_at
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn find_user_by_refresh_token(
        &self,
        token_hash: &str,
    ) -> Result<Option<Uuid>, anyhow::Error> {
        let row = sqlx::query_scalar!(
            "SELECT user_id FROM auth.refresh_tokens
             WHERE token_hash = $1 AND expires_at > now()",
            token_hash
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn revoke_refresh_token(&self, token_hash: &str) -> Result<(), anyhow::Error> {
        sqlx::query!(
            "DELETE FROM auth.refresh_tokens WHERE token_hash = $1",
            token_hash
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn revoke_all_refresh_tokens(&self, user_id: Uuid) -> Result<(), anyhow::Error> {
        sqlx::query!(
            "DELETE FROM auth.refresh_tokens WHERE user_id = $1",
            user_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn save_otp(
        &self,
        user_id: Uuid,
        otp_hash: &str,
        purpose: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), anyhow::Error> {
        sqlx::query!(
            "INSERT INTO auth.otp_verifications (id, user_id, otp_hash, purpose, expires_at, attempts)
             VALUES (gen_random_uuid(), $1, $2, $3, $4, 0)
             ON CONFLICT (user_id, purpose)
             DO UPDATE SET otp_hash = $2, expires_at = $4, attempts = 0",
            user_id, otp_hash, purpose, expires_at
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn consume_otp(
        &self,
        user_id: Uuid,
        otp_hash: &str,
        purpose: &str,
    ) -> Result<bool, anyhow::Error> {
        let result = sqlx::query!(
            "DELETE FROM auth.otp_verifications
             WHERE user_id = $1 AND otp_hash = $2 AND purpose = $3
               AND expires_at > now()",
            user_id,
            otp_hash,
            purpose
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn bump_otp_attempts(
        &self,
        user_id: Uuid,
        purpose: &str,
        max_attempts: i32,
    ) -> Result<i32, anyhow::Error> {
        // Increment attempts; kembalikan nilai baru (0 bila tidak ada OTP aktif).
        let attempts: i32 = sqlx::query_scalar!(
            "UPDATE auth.otp_verifications
             SET attempts = attempts + 1
             WHERE user_id = $1 AND purpose = $2
             RETURNING attempts",
            user_id,
            purpose
        )
        .fetch_optional(&self.pool)
        .await?
        .unwrap_or(0);

        // Bila melampaui batas, hapus OTP agar tidak bisa di-brute-force lagi.
        if attempts >= max_attempts {
            sqlx::query!(
                "DELETE FROM auth.otp_verifications WHERE user_id = $1 AND purpose = $2",
                user_id,
                purpose
            )
            .execute(&self.pool)
            .await?;
        }
        Ok(attempts)
    }

    async fn set_status(&self, user_id: Uuid, status: AccountStatus) -> Result<(), anyhow::Error> {
        sqlx::query!(
            "UPDATE auth.users SET status = $2, updated_at = now() WHERE id = $1",
            user_id,
            status.as_str()
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_status(&self, user_id: Uuid) -> Result<Option<AccountStatus>, anyhow::Error> {
        let row = sqlx::query_scalar!("SELECT status FROM auth.users WHERE id = $1", user_id)
            .fetch_optional(&self.pool)
            .await?;

        row.map(|s| parse_status(&s)).transpose()
    }

    async fn update_password(
        &self,
        user_id: Uuid,
        password_hash: &str,
    ) -> Result<(), anyhow::Error> {
        sqlx::query!(
            "UPDATE auth.users SET password_hash = $2, updated_at = now() WHERE id = $1",
            user_id,
            password_hash
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn insert_suspension(
        &self,
        user_id: Uuid,
        is_permanent: bool,
        reason: &str,
        expires_at: Option<DateTime<Utc>>,
        created_by: Uuid,
    ) -> Result<(), anyhow::Error> {
        sqlx::query!(
            "INSERT INTO auth.account_suspension (id, user_id, is_permanent, reason, expires_at, created_by)
             VALUES (gen_random_uuid(), $1, $2, $3, $4, $5)",
            user_id, is_permanent, reason, expires_at, created_by
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn has_active_suspension(&self, user_id: Uuid) -> Result<bool, anyhow::Error> {
        Ok(sqlx::query_scalar!(
            "SELECT EXISTS(
                SELECT 1 FROM auth.account_suspension
                WHERE user_id = $1
                  AND (is_permanent = true OR (expires_at IS NOT NULL AND expires_at > now()))
            )",
            user_id
        )
        .fetch_one(&self.pool)
        .await?
        .unwrap_or(false))
    }
}
