use std::sync::Arc;

use anyhow::{anyhow, Context};
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::Utc;
use uuid::Uuid;

use super::dto::{
    AdminLoginInput, ChangePasswordInput, LoginInput, RefreshInput, RegisterInput, ResendOtpInput,
    ResetPasswordInput, TokenPair, VerifyOtpInput,
};
use crate::domain::entity::{AccountStatus, OtpPurpose};
use crate::domain::rate_limit::RateLimiter;
use crate::domain::repository::AuthRepository;
use crate::domain::token::TokenIssuer;
use notification_service_client::{EmailMessage, NotificationClient};

/// TTL OTP dalam menit (RQ2 — default usulan).
const OTP_TTL_MINUTES: i64 = 5;
/// Maks percobaan verifikasi OTP sebelum OTP dibatalkan (RQ2).
const MAX_OTP_ATTEMPTS: i32 = 5;
/// Versi T&C default bila user tidak menyertakan.
const DEFAULT_TOS_VERSION: &str = "v1";
/// TTL refresh token default (30 hari) — dipakai sebagai fallback di router.
pub const DEFAULT_REFRESH_TTL_SECS: i64 = 2_592_000;
/// TTL access token default (15 menit) — sumber tunggal untuk JwtService dan TokenPair.expires_in.
pub const DEFAULT_ACCESS_TTL_SECS: i64 = 900;
/// Durasi access token dalam u64 — cast dari DEFAULT_ACCESS_TTL_SECS.
const ACCESS_TOKEN_EXPIRY_SECS: u64 = DEFAULT_ACCESS_TTL_SECS as u64;
/// Maks percobaan login per email dalam window rate limit (anti brute-force).
const LOGIN_RATE_LIMIT_MAX: i64 = 5;
/// Jendela rate limit login dalam detik (15 menit).
const LOGIN_RATE_LIMIT_WINDOW_SECS: i64 = 15 * 60;
/// Dummy bcrypt hash untuk constant-time admin_login (timing side-channel protection).
/// Nilai ini hanya digunakan untuk padding timing, bukan untuk perbandingan kredensial.
const DUMMY_BCRYPT_HASH: &str = "$2b$12$LJ3m4ys3Lk0TSwHCpNqrAOZBXK8mB3yZF5s0HVMCJzmVCAg8FwvKe";
/// Kategori storage untuk bukti penangguhan.
pub mod storage_category {
    pub const SUSPENSION_EVIDENCE: &str = "suspension-evidence";
}

/// Teks notifikasi suspend (D5) — disentralisasi agar zero hardcoded di alur bisnis.
mod suspend_notice {
    pub const SUBJECT: &str = "Pemberitahuan Penangguhan Akun Rejki";
    pub const TITLE_PERMANENT: &str = "Akun Anda telah ditangguhkan secara permanen";
    pub const TITLE_TEMP: &str = "Akun Anda telah ditangguhkan sementara";
    pub const TAIL_PERMANENT: &str = "Silakan hubungi admin untuk informasi lebih lanjut.";
    pub const TAIL_TEMP: &str =
        "Anda akan dapat mengakses kembali setelah masa penangguhan berakhir.";
}

/// Domain error type untuk AuthService — memungkinkan handler membedakan
/// rate-limit (→ 429) dari auth failure (→ 401) dari error internal (→ 500).
#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("{0}")]
    RateLimited(String),
    #[error("{0}")]
    Unauthorized(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Param suspend satu akun — grouping untuk menghindari too_many_arguments (7 param
/// di `suspend_account`). Mirror dari `BulkSuspendParams` di bawah.
pub struct SuspendAccountParams<'a> {
    pub user_id: Uuid,
    pub permanent: bool,
    pub reason: &'a str,
    pub expires_at: Option<chrono::DateTime<Utc>>,
    pub admin_id: Uuid,
    pub evidence_object_key: Option<&'a str>,
    pub notifier: Option<&'a dyn NotificationClient>,
}

/// Param bulk suspend — grouping untuk menghindari too_many_arguments (9 param
/// di `suspend_accounts_bulk`). Semua field hidup selama pemrosesan batch.
pub struct BulkSuspendParams<'a> {
    pub user_ids: &'a [Uuid],
    pub permanent: bool,
    pub reason: &'a str,
    pub expires_at: Option<chrono::DateTime<Utc>>,
    pub admin_id: Uuid,
    pub evidence_object_key: Option<&'a str>,
    pub notifier: Option<&'a dyn NotificationClient>,
    pub user_client: Option<&'a dyn user_service_client::UserClient>,
}

pub struct AuthService<R: AuthRepository> {
    repo: Arc<R>,
    token_issuer: Arc<dyn TokenIssuer>,
    refresh_ttl: i64, // detik
    rate_limiter: Arc<dyn RateLimiter>,
    /// Kontrak pengiriman notifikasi (email OTP). In-process di monolith;
    /// di-inject dari Composition Root. Opsional agar standalone/test tetap jalan.
    notifier: Option<Arc<dyn NotificationClient>>,
}

impl<R: AuthRepository> AuthService<R> {
    pub fn new(
        repo: Arc<R>,
        token_issuer: Arc<dyn TokenIssuer>,
        refresh_ttl: i64,
        rate_limiter: Arc<dyn RateLimiter>,
    ) -> Self {
        Self {
            repo,
            token_issuer,
            refresh_ttl,
            rate_limiter,
            notifier: None,
        }
    }

    /// Inject NotificationClient (dipanggil dari Composition Root). Builder agar
    /// `new()` lama tetap kompatibel untuk standalone/test.
    pub fn with_notifier(mut self, notifier: Arc<dyn NotificationClient>) -> Self {
        self.notifier = Some(notifier);
        self
    }

    /// Rate limit check untuk endpoint login (anti brute-force).
    /// Key: `login_req:{endpoint}:{sha256(email)}`. Fail-open bila Redis down.
    async fn check_login_rate_limit(
        &self,
        endpoint: &str,
        email: &str,
    ) -> Result<(), ServiceError> {
        let email_hash = sha256_hex(email);
        let key = format!("login_req:{endpoint}:{email_hash}");
        let allowed = self
            .rate_limiter
            .allow_raw(&key, LOGIN_RATE_LIMIT_MAX, LOGIN_RATE_LIMIT_WINDOW_SECS)
            .await;
        if allowed {
            Ok(())
        } else {
            Err(ServiceError::RateLimited(
                "terlalu banyak percobaan login, coba lagi nanti".into(),
            ))
        }
    }

    /// Kirim OTP ke email lewat kontrak NotificationClient (domain notification yang
    /// memegang detail Redis/SMTP). Bila notifier tidak di-inject atau gagal, log
    /// peringatan tanpa membocorkan OTP dan tanpa menggagalkan alur utama.
    /// Email di-hash (SHA-256) di log untuk kepatuhan PII/GDPR.
    async fn dispatch_otp_email(&self, to: &str, purpose: &str, otp: &str) {
        let email_hash = sha256_hex(to);
        let Some(notifier) = &self.notifier else {
            tracing::warn!(
                email_hash = %email_hash,
                purpose,
                "notifier tidak dikonfigurasi — email OTP tidak dikirim"
            );
            return;
        };
        let body = format!(
            "Kode OTP Anda untuk {purpose}: {otp}\nBerlaku {OTP_TTL_MINUTES} menit. Jangan bagikan kode ini."
        );
        let msg = EmailMessage {
            to: to.to_owned(),
            subject: "Kode OTP Rejki".to_owned(),
            body,
        };
        if let Err(e) = notifier.send_email(msg).await {
            tracing::warn!(error = %e, email_hash = %email_hash, purpose, "gagal kirim email OTP via notifier");
        }
    }

    pub async fn register(&self, input: RegisterInput) -> Result<(), anyhow::Error> {
        // Catatan: persetujuan T&C & kekuatan password sudah divalidasi di DTO (ValidatedJson → 422).

        // Anti-enumeration: jika email sudah ada, jangan bocorkan. Untuk akun yang masih
        // pending_verification, kirim ulang OTP; selain itu balas sukses generik tanpa aksi.
        if let Some(existing) = self.repo.find_by_email(&input.email).await? {
            if existing.status == AccountStatus::PendingVerification {
                let otp = generate_otp();
                let otp_hash = sha256_hex(&otp);
                let expires_at = Utc::now() + chrono::Duration::minutes(OTP_TTL_MINUTES);
                self.repo
                    .save_otp(
                        existing.id,
                        &otp_hash,
                        OtpPurpose::Register.as_str(),
                        expires_at,
                    )
                    .await?;
                self.dispatch_otp_email(&input.email, OtpPurpose::Register.as_str(), &otp)
                    .await;
            }
            return Ok(());
        }

        let password_hash = hash(&input.password, DEFAULT_COST).context("gagal hash password")?;

        // phone dienkripsi at-rest (AES-256-GCM, K14) sebelum disimpan.
        let phone_encrypted = match input.phone.as_deref() {
            Some(p) => Some(crate::application::crypto::encrypt(p)?),
            None => None,
        };
        let tos_version = input.tos_version.as_deref().unwrap_or(DEFAULT_TOS_VERSION);

        let user = self
            .repo
            .create_user(
                &input.email,
                &password_hash,
                phone_encrypted.as_deref(),
                tos_version,
            )
            .await?;

        let otp = generate_otp();
        let otp_hash = sha256_hex(&otp);
        let expires_at = Utc::now() + chrono::Duration::minutes(OTP_TTL_MINUTES);

        self.repo
            .save_otp(
                user.id,
                &otp_hash,
                OtpPurpose::Register.as_str(),
                expires_at,
            )
            .await?;

        self.dispatch_otp_email(&input.email, OtpPurpose::Register.as_str(), &otp)
            .await;

        Ok(())
    }

    pub async fn verify_otp(&self, input: VerifyOtpInput) -> Result<(), anyhow::Error> {
        let user = self
            .repo
            .find_by_email(&input.email)
            .await?
            .ok_or_else(|| anyhow!("user tidak ditemukan"))?;

        let otp_hash = sha256_hex(&input.otp);
        let consumed = self
            .repo
            .consume_otp(user.id, &otp_hash, &input.purpose)
            .await?;

        if !consumed {
            // Catat percobaan gagal; OTP dibatalkan bila melampaui batas (anti brute-force).
            // Transactional: mencegah race condition dengan concurrent save_otp (resend).
            self.repo
                .bump_otp_attempts_transactional(user.id, &input.purpose, MAX_OTP_ATTEMPTS)
                .await?;
            return Err(anyhow!("OTP tidak valid atau sudah expired"));
        }

        // Verifikasi OTP register menaikkan status: pending_verification -> profile_incomplete.
        if input.purpose == OtpPurpose::Register.as_str()
            && user.status == AccountStatus::PendingVerification
        {
            self.transition_status(user.id, user.status, AccountStatus::ProfileIncomplete)
                .await?;
        }

        Ok(())
    }

    /// Ubah status akun setelah memvalidasi transisi pada state machine.
    pub async fn transition_status(
        &self,
        user_id: Uuid,
        from: AccountStatus,
        to: AccountStatus,
    ) -> Result<(), anyhow::Error> {
        if !from.can_transition_to(to) {
            return Err(anyhow!(
                "transisi status tidak sah: {} -> {}",
                from.as_str(),
                to.as_str()
            ));
        }
        self.repo.set_status(user_id, to).await
    }

    /// Ambil status akun terkini (dipakai AuthClient.get_account_status).
    pub async fn get_status(&self, user_id: Uuid) -> Result<Option<AccountStatus>, anyhow::Error> {
        self.repo.get_status(user_id).await
    }

    /// Set status akun dengan validasi transisi (dipakai AuthClient.set_account_status).
    pub async fn set_status_checked(
        &self,
        user_id: Uuid,
        to: AccountStatus,
    ) -> Result<(), anyhow::Error> {
        let current = self
            .repo
            .get_status(user_id)
            .await?
            .ok_or_else(|| anyhow!("user tidak ditemukan"))?;
        self.transition_status(user_id, current, to).await
    }

    /// Check if a SuspendedTemp account can be auto-recovered (suspension expired).
    /// Returns Some(Active) if auto-recovered, None if still suspended (caller should reject).
    /// Shared by login and admin_login to eliminate copy-paste (D10).
    async fn auto_recover_suspension(
        &self,
        user_id: Uuid,
    ) -> Result<Option<AccountStatus>, anyhow::Error> {
        if self.repo.has_active_suspension(user_id).await? {
            Ok(None) // still suspended
        } else {
            self.repo.set_status(user_id, AccountStatus::Active).await?;
            Ok(Some(AccountStatus::Active))
        }
    }

    pub async fn login(&self, input: LoginInput) -> Result<TokenPair, ServiceError> {
        // Rate limit anti brute-force — gagal tanpa membocorkan keberadaan akun.
        self.check_login_rate_limit("login", &input.email).await?;

        let user = self
            .repo
            .find_by_email(&input.email)
            .await
            .map_err(ServiceError::Other)?
            .ok_or_else(|| ServiceError::Unauthorized("email atau password salah".into()))?;

        // Verifikasi password DAHULU sebelum mutasi status apapun.
        // Mencegah account state mutation tanpa authentication success.
        if !verify(&input.password, &user.password_hash)
            .context("gagal verifikasi password")
            .map_err(ServiceError::Other)?
        {
            tracing::warn!(
                email_hash = %sha256_hex(&input.email),
                "login failed: invalid credentials"
            );
            return Err(ServiceError::Unauthorized(
                "email atau password salah".into(),
            ));
        }

        // Cek kelayakan akun via status (menggantikan is_verified).
        // Setelah password verified, baru evaluasi status.
        let effective_status = match user.status {
            AccountStatus::PendingVerification => {
                tracing::warn!(
                    email_hash = %sha256_hex(&input.email),
                    "login failed: account not verified"
                );
                return Err(ServiceError::Unauthorized(
                    "email atau password salah".into(),
                ));
            }
            AccountStatus::SuspendedPermanent => {
                tracing::warn!(
                    email_hash = %sha256_hex(&input.email),
                    "login failed: account suspended permanently"
                );
                return Err(ServiceError::Unauthorized(
                    "email atau password salah".into(),
                ));
            }
            AccountStatus::SuspendedTemp => {
                // Auto-pulih: jika tidak ada penangguhan yang masih berlaku, kembalikan ke active.
                match self
                    .auto_recover_suspension(user.id)
                    .await
                    .map_err(ServiceError::Other)?
                {
                    Some(AccountStatus::Active) => AccountStatus::Active,
                    _ => {
                        tracing::warn!(
                            email_hash = %sha256_hex(&input.email),
                            "login failed: account suspended temporarily"
                        );
                        return Err(ServiceError::Unauthorized(
                            "email atau password salah".into(),
                        ));
                    }
                }
            }
            other => other,
        };

        // Audit: login sukses tercatat.
        tracing::info!(
            user_id = %user.id,
            email_hash = %sha256_hex(&user.email),
            "login berhasil"
        );

        self.issue_tokens(&user, effective_status)
            .await
            .map_err(ServiceError::Other)
    }

    /// Admin login: verifikasi email+password, cek `role=admin` + `status=active`.
    /// Anti-enumeration: pesan galat seragam untuk kredensial salah, non-admin,
    /// akun tidak ditemukan — tidak membocorkan keberadaan akun.
    /// Timing side-channel: bcrypt verify SELALU dijalankan (dengan dummy hash
    /// bila user tidak ditemukan atau non-admin) untuk response time konstan ~250ms.
    /// SuspendedTemp auto-recovery: jika masa suspensi sementara admin sudah expired,
    /// status dikembalikan ke Active (simetris dengan login reguler).
    /// Ref: openspec/changes/add-admin-rbac D3.
    pub async fn admin_login(&self, input: AdminLoginInput) -> Result<TokenPair, ServiceError> {
        // Rate limit anti brute-force — gagal tanpa membocorkan keberadaan akun.
        self.check_login_rate_limit("admin_login", &input.email)
            .await?;

        // Cari user; jika tidak ada, jalankan bcrypt dengan dummy hash agar
        // timing konstan (anti-enumeration via response time).
        let user_opt = self
            .repo
            .find_by_email(&input.email)
            .await
            .map_err(ServiceError::Other)?;
        let (is_admin, status, password_hash) = match &user_opt {
            Some(u) => (u.role.is_admin(), u.status, u.password_hash.clone()),
            None => (false, AccountStatus::Active, DUMMY_BCRYPT_HASH.to_owned()),
        };

        // bcrypt verify SELALU dijalankan — ~250ms untuk semua jalur.
        let password_ok = verify(&input.password, &password_hash)
            .context("gagal verifikasi password")
            .map_err(ServiceError::Other)?;

        // Evaluasi semua kondisi SETELAH bcrypt verify (constant-time path).
        if !password_ok || user_opt.is_none() || !is_admin {
            tracing::warn!(
                email_hash = %sha256_hex(&input.email),
                "admin login failed: invalid credentials or not admin"
            );
            return Err(ServiceError::Unauthorized(
                "email atau password salah".into(),
            ));
        }

        let user = user_opt.unwrap(); // safe: semua guard di atas terpenuhi

        // Evaluasi status — dengan auto-recovery untuk SuspendedTemp expired.
        let effective_status = match status {
            AccountStatus::Active => AccountStatus::Active,
            AccountStatus::SuspendedTemp => {
                match self
                    .auto_recover_suspension(user.id)
                    .await
                    .map_err(ServiceError::Other)?
                {
                    Some(AccountStatus::Active) => AccountStatus::Active,
                    _ => {
                        tracing::warn!(
                            email_hash = %sha256_hex(&input.email),
                            "admin login failed: account suspended"
                        );
                        return Err(ServiceError::Unauthorized(
                            "email atau password salah".into(),
                        ));
                    }
                }
            }
            _ => {
                tracing::warn!(
                    email_hash = %sha256_hex(&input.email),
                    "admin login failed: account not active"
                );
                return Err(ServiceError::Unauthorized(
                    "email atau password salah".into(),
                ));
            }
        };

        // Audit: login admin tercatat.
        tracing::info!(
            user_id = %user.id,
            email_hash = %sha256_hex(&user.email),
            "admin login berhasil"
        );

        self.issue_tokens(&user, effective_status)
            .await
            .map_err(ServiceError::Other)
    }

    /// Shared helper: issue JWT + refresh token + save ke DB.
    /// Dipakai oleh `login`, `admin_login`, dan `refresh`.
    async fn issue_tokens(
        &self,
        user: &crate::domain::entity::AuthUser,
        effective_status: crate::domain::entity::AccountStatus,
    ) -> Result<TokenPair, anyhow::Error> {
        let access_token = self.token_issuer.issue_access_token(
            user.id,
            &user.email,
            effective_status,
            user.role,
        )?;
        let refresh_token = generate_refresh_token();
        let refresh_hash = sha256_hex(&refresh_token);
        let expires_at = Utc::now() + chrono::Duration::seconds(self.refresh_ttl);
        self.repo
            .save_refresh_token(user.id, &refresh_hash, expires_at)
            .await?;
        Ok(TokenPair {
            access_token,
            refresh_token,
            token_type: "Bearer".into(),
            expires_in: ACCESS_TOKEN_EXPIRY_SECS,
        })
    }

    pub async fn refresh(&self, input: RefreshInput) -> Result<TokenPair, anyhow::Error> {
        let token_hash = sha256_hex(&input.refresh_token);

        // DELETE-first atomic: hapus token DAN dapatkan user_id dalam satu operasi.
        // SQL sudah memfilter expires_at > now() — token expired tidak akan match.
        // Mencegah race condition di mana dua request konkuren dengan token yang sama
        // sama-sama lolos lookup dan masing-masing menerbitkan token baru.
        let (user_id, expires_at) = self
            .repo
            .delete_and_return_refresh_token(&token_hash)
            .await?
            .ok_or_else(|| anyhow!("refresh token tidak valid atau sudah expired"))?;

        // Verifikasi expiry di service layer sebagai lapis pertahanan ganda
        // (defense-in-depth — SQL sudah memfilter, ini safety net).
        if expires_at <= Utc::now() {
            return Err(anyhow!("refresh token sudah expired"));
        }

        let user = self
            .repo
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| anyhow!("user tidak ditemukan"))?;

        self.issue_tokens(&user, user.status).await
    }

    pub async fn logout(&self, refresh_token: &str) -> Result<(), anyhow::Error> {
        let token_hash = sha256_hex(refresh_token);
        self.repo.revoke_refresh_token(&token_hash).await
    }

    pub async fn resend_otp(&self, input: ResendOtpInput) -> Result<(), ServiceError> {
        let key = format!("otp_req:{}:{}", &input.purpose, &input.email);
        if !self.rate_limiter.allow_raw(&key, 3, 15 * 60).await {
            return Err(ServiceError::RateLimited(
                "terlalu banyak permintaan OTP, coba lagi nanti".into(),
            ));
        }
        let user = self
            .repo
            .find_by_email(&input.email)
            .await
            .map_err(ServiceError::Other)?
            .ok_or_else(|| ServiceError::Unauthorized("user tidak ditemukan".into()))?;

        let otp = generate_otp();
        let otp_hash = sha256_hex(&otp);
        let expires_at = Utc::now() + chrono::Duration::minutes(OTP_TTL_MINUTES);

        self.repo
            .save_otp(user.id, &otp_hash, &input.purpose, expires_at)
            .await?;

        self.dispatch_otp_email(&input.email, &input.purpose, &otp)
            .await;

        Ok(())
    }

    /// US-05: minta reset password. SELALU balas Ok (anti-enumeration) — bila email
    /// terdaftar, kirim OTP `reset_password`; bila tidak, tidak melakukan apa-apa.
    pub async fn forgot_password(&self, email: &str) -> Result<(), anyhow::Error> {
        // Rate-limit per email demi anti-spam (tetap balas Ok agar anti-enumeration).
        let key = format!("otp_req:{}:{}", OtpPurpose::ResetPassword.as_str(), email);
        if !self.rate_limiter.allow_raw(&key, 3, 15 * 60).await {
            return Ok(());
        }
        if let Some(user) = self.repo.find_by_email(email).await? {
            let otp = generate_otp();
            let otp_hash = sha256_hex(&otp);
            let expires_at = Utc::now() + chrono::Duration::minutes(OTP_TTL_MINUTES);
            self.repo
                .save_otp(
                    user.id,
                    &otp_hash,
                    OtpPurpose::ResetPassword.as_str(),
                    expires_at,
                )
                .await?;
            self.dispatch_otp_email(email, OtpPurpose::ResetPassword.as_str(), &otp)
                .await;
        }
        Ok(())
    }

    /// US-05: set password baru dengan OTP `reset_password`. Cabut SEMUA refresh token
    /// dalam transaction atomik — consume OTP + revoke token + update password dalam
    /// satu unit atomik. Mencegah OTP terlanjur dikonsumsi walau password update gagal.
    pub async fn reset_password(&self, input: ResetPasswordInput) -> Result<(), anyhow::Error> {
        let user = self
            .repo
            .find_by_email(&input.email)
            .await?
            .ok_or_else(|| anyhow!("OTP tidak valid atau sudah expired"))?;

        let otp_hash = sha256_hex(&input.otp);
        let new_hash = hash(&input.new_password, DEFAULT_COST).context("gagal hash password")?;

        // Atomik: consume OTP + revoke token + update password dalam satu transaction.
        let consumed = self
            .repo
            .consume_otp_and_update_password_transactional(
                user.id,
                &otp_hash,
                OtpPurpose::ResetPassword.as_str(),
                &new_hash,
            )
            .await?;

        if !consumed {
            // OTP tidak valid — catat percobaan gagal (dalam transaction terpisah).
            self.repo
                .bump_otp_attempts_transactional(
                    user.id,
                    OtpPurpose::ResetPassword.as_str(),
                    MAX_OTP_ATTEMPTS,
                )
                .await?;
            return Err(anyhow!("OTP tidak valid atau sudah expired"));
        }

        tracing::info!(
            user_id = %user.id,
            "password reset successful"
        );
        Ok(())
    }

    /// US-06: ubah password oleh user terautentikasi dengan OTP `change_password`.
    /// Cabut SEMUA refresh token (sesi lain) dalam transaction atomik —
    /// consume OTP + revoke token + update password dalam satu unit atomik.
    pub async fn change_password(
        &self,
        user_id: Uuid,
        input: ChangePasswordInput,
    ) -> Result<(), anyhow::Error> {
        let otp_hash = sha256_hex(&input.otp);
        let new_hash = hash(&input.new_password, DEFAULT_COST).context("gagal hash password")?;

        // Atomik: consume OTP + revoke token + update password dalam satu transaction.
        let consumed = self
            .repo
            .consume_otp_and_update_password_transactional(
                user_id,
                &otp_hash,
                OtpPurpose::ChangePassword.as_str(),
                &new_hash,
            )
            .await?;

        if !consumed {
            self.repo
                .bump_otp_attempts_transactional(
                    user_id,
                    OtpPurpose::ChangePassword.as_str(),
                    MAX_OTP_ATTEMPTS,
                )
                .await?;
            return Err(anyhow!("OTP tidak valid atau sudah expired"));
        }

        tracing::info!(
            user_id = %user_id,
            "password change successful"
        );
        Ok(())
    }

    /// Minta OTP `change_password` untuk user terautentikasi (dikirim ke email terdaftar).
    pub async fn request_change_password_otp(&self, user_id: Uuid) -> Result<(), ServiceError> {
        let key = format!(
            "otp_req:{}:{}",
            OtpPurpose::ChangePassword.as_str(),
            &user_id.to_string()
        );
        if !self.rate_limiter.allow_raw(&key, 3, 15 * 60).await {
            return Err(ServiceError::RateLimited(
                "terlalu banyak permintaan OTP, coba lagi nanti".into(),
            ));
        }
        let user = self
            .repo
            .find_by_id(user_id)
            .await
            .map_err(ServiceError::Other)?
            .ok_or_else(|| ServiceError::Unauthorized("user tidak ditemukan".into()))?;
        let otp = generate_otp();
        let otp_hash = sha256_hex(&otp);
        let expires_at = Utc::now() + chrono::Duration::minutes(OTP_TTL_MINUTES);
        self.repo
            .save_otp(
                user.id,
                &otp_hash,
                OtpPurpose::ChangePassword.as_str(),
                expires_at,
            )
            .await?;
        self.dispatch_otp_email(&user.email, OtpPurpose::ChangePassword.as_str(), &otp)
            .await;
        Ok(())
    }

    /// US-07: tangguhkan akun (sementara dengan `expires_at`, atau permanen bila None).
    /// Mengubah status, mencatat riwayat+audit, mencabut seluruh refresh token,
    /// DAN mengirim notifikasi email + in-app (best-effort).
    pub async fn suspend_account(
        &self,
        params: SuspendAccountParams<'_>,
    ) -> Result<(), anyhow::Error> {
        let email = self
            .suspend_one(
                params.user_id,
                params.permanent,
                params.reason,
                params.expires_at,
                params.admin_id,
                params.evidence_object_key,
            )
            .await?;

        // Kirim notifikasi (best-effort, konsisten dengan bulk path).
        if let Some(n) = params.notifier {
            self.notify_suspended(n, params.user_id, &email, params.permanent, params.reason)
                .await;
        }

        Ok(())
    }

    /// Suspend satu pengguna — reusable untuk single endpoint & bulk (D3).
    /// Memeriksa keberadaan user dahulu agar nonexistent-ID menghasilkan error
    /// (tidak silent success seperti UPDATE dengan 0 rows_affected).
    /// Memvalidasi transisi state machine (D1/account-status-lifecycle).
    /// Tiga operasi write (set_status, insert_suspension, revoke_all_refresh_tokens)
    /// dibungkus transaction untuk atomic rollback pada partial failure.
    /// Mengembalikan email pengguna agar pemanggil dapat mengirim notifikasi email (D5).
    async fn suspend_one(
        &self,
        user_id: Uuid,
        permanent: bool,
        reason: &str,
        expires_at: Option<chrono::DateTime<Utc>>,
        admin_id: Uuid,
        evidence_object_key: Option<&str>,
    ) -> Result<String, anyhow::Error> {
        // Cek keberadaan user + validasi state machine.
        let user = self
            .repo
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| anyhow!("user tidak ditemukan"))?;

        let target = if permanent {
            AccountStatus::SuspendedPermanent
        } else {
            AccountStatus::SuspendedTemp
        };

        // Validasi transisi state machine — hanya Active yang boleh di-suspend.
        if !user.status.can_transition_to(target) {
            return Err(anyhow!(
                "transisi status tidak sah: {} -> {}",
                user.status.as_str(),
                target.as_str()
            ));
        }

        // Transaction wrapping: set_status + insert_suspension + revoke_all tokens
        // dalam satu unit atomik — gagal di tengah → rollback otomatis.
        self.repo
            .suspend_user_transactional(
                user_id,
                target,
                permanent,
                reason,
                expires_at,
                admin_id,
                evidence_object_key,
            )
            .await?;

        tracing::info!(
            user_id = %user_id, admin_id = %admin_id, permanent, reason, evidence_object_key,
            "account suspended"
        );
        Ok(user.email)
    }

    /// Kirim notifikasi suspend ke pengguna: in-app + email (D5).
    /// Best-effort + logged — kegagalan notifikasi tidak menggagalkan suspend.
    async fn notify_suspended(
        &self,
        notifier: &dyn NotificationClient,
        user_id: Uuid,
        email: &str,
        permanent: bool,
        reason: &str,
    ) {
        let (title, tail) = if permanent {
            (
                suspend_notice::TITLE_PERMANENT,
                suspend_notice::TAIL_PERMANENT,
            )
        } else {
            (suspend_notice::TITLE_TEMP, suspend_notice::TAIL_TEMP)
        };
        let body = format!("Alasan: {reason}\n{tail}");

        // In-app.
        if let Err(e) = notifier
            .send(
                user_id,
                notification_service_client::NotificationPayload {
                    title: title.to_owned(),
                    body: body.clone(),
                    data: Some(serde_json::json!({
                        "type": "account_suspended",
                        "permanent": permanent,
                    })),
                },
            )
            .await
        {
            tracing::warn!(user_id = %user_id, error = %e, "gagal kirim notifikasi in-app suspend");
        }

        // Email.
        if let Err(e) = notifier
            .send_email(EmailMessage {
                to: email.to_owned(),
                subject: suspend_notice::SUBJECT.to_owned(),
                body: format!("{title}\n\n{body}"),
            })
            .await
        {
            tracing::warn!(user_id = %user_id, error = %e, "gagal kirim email suspend");
        }
    }

    /// Bulk suspend pengguna (extend-user-suspension-bulk-purge, D1-D3).
    /// Validasi fail-fast pada level permintaan; tiap item diproses independen
    /// dengan hasil partial-success (konsisten dengan SuspendResponse iklan).
    /// Param di-group ke struct untuk menghindari too_many_arguments (clippy 9/7).
    pub async fn suspend_accounts_bulk(
        &self,
        params: BulkSuspendParams<'_>,
    ) -> Result<Vec<crate::application::dto::BulkSuspendResultItem>, anyhow::Error> {
        let mut results = Vec::with_capacity(params.user_ids.len());

        for &user_id in params.user_ids {
            match self
                .suspend_one(
                    user_id,
                    params.permanent,
                    params.reason,
                    params.expires_at,
                    params.admin_id,
                    params.evidence_object_key,
                )
                .await
            {
                Ok(email) => {
                    // Purge dokumen saat suspend permanen (D4) — best-effort + logged.
                    if params.permanent {
                        if let Some(uc) = params.user_client {
                            if let Err(e) = uc.purge_kyc_documents(user_id).await {
                                tracing::warn!(
                                    user_id = %user_id,
                                    error = %e,
                                    "purge dokumen gagal saat suspend permanen — perlu pembersihan lanjutan"
                                );
                            }
                        }
                    }

                    // Notifikasi email + in-app (D5) — best-effort + logged.
                    if let Some(n) = params.notifier {
                        self.notify_suspended(n, user_id, &email, params.permanent, params.reason)
                            .await;
                    }

                    results.push(crate::application::dto::BulkSuspendResultItem {
                        user_id,
                        success: true,
                        error: None,
                    });
                }
                Err(e) => {
                    results.push(crate::application::dto::BulkSuspendResultItem {
                        user_id,
                        success: false,
                        error: Some(e.to_string()),
                    });
                }
            }
        }

        Ok(results)
    }
}

fn generate_otp() -> String {
    use rand::Rng;
    format!("{:06}", rand::rng().random_range(0..1_000_000))
}

fn generate_refresh_token() -> String {
    use rand::Rng;
    let bytes: Vec<u8> = (0..32).map(|_| rand::rng().random()).collect();
    hex::encode(bytes)
}

/// SHA-256 hex digest helper — dipakai untuk OTP hashing dan refresh token hashing.
fn sha256_hex(input: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(input.as_bytes());
    format!("{:x}", h.finalize())
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entity::{AccountStatus, AuthUser, OtpPurpose, Role};
    use crate::domain::repository::AuthRepository;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// Simple in-memory mock of AuthRepository for unit testing.
    #[allow(dead_code)]
    struct MockAuthRepository {
        users: Mutex<HashMap<Uuid, AuthUser>>,
        refresh_tokens: Mutex<HashMap<String, Uuid>>, // token_hash -> user_id
        otps: Mutex<HashMap<String, (String, i32)>>,  // "user_id:purpose" -> (otp_hash, attempts)
    }

    impl MockAuthRepository {
        fn new() -> Self {
            Self {
                users: Mutex::new(HashMap::new()),
                refresh_tokens: Mutex::new(HashMap::new()),
                otps: Mutex::new(HashMap::new()),
            }
        }

        fn insert_user(&self, user: AuthUser) {
            self.users.lock().unwrap().insert(user.id, user);
        }
    }

    #[allow(async_fn_in_trait)]
    impl AuthRepository for MockAuthRepository {
        async fn find_by_email(&self, email: &str) -> Result<Option<AuthUser>, anyhow::Error> {
            Ok(self
                .users
                .lock()
                .unwrap()
                .values()
                .find(|u| u.email == email)
                .cloned())
        }

        async fn find_by_id(&self, id: Uuid) -> Result<Option<AuthUser>, anyhow::Error> {
            Ok(self.users.lock().unwrap().get(&id).cloned())
        }

        async fn create_user(
            &self,
            email: &str,
            password_hash: &str,
            _phone_encrypted: Option<&str>,
            _tos_version: &str,
        ) -> Result<AuthUser, anyhow::Error> {
            let user = AuthUser {
                id: Uuid::now_v7(),
                email: email.to_owned(),
                password_hash: password_hash.to_owned(),
                status: AccountStatus::PendingVerification,
                role: Role::User,
                phone: None,
                tos_accepted_at: Some(chrono::Utc::now()),
                tos_version: Some("v1".into()),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };
            self.users.lock().unwrap().insert(user.id, user.clone());
            Ok(user)
        }

        async fn save_refresh_token(
            &self,
            user_id: Uuid,
            token_hash: &str,
            _expires_at: chrono::DateTime<Utc>,
        ) -> Result<(), anyhow::Error> {
            self.refresh_tokens
                .lock()
                .unwrap()
                .insert(token_hash.to_owned(), user_id);
            Ok(())
        }

        async fn find_user_by_refresh_token(
            &self,
            token_hash: &str,
        ) -> Result<Option<Uuid>, anyhow::Error> {
            Ok(self.refresh_tokens.lock().unwrap().get(token_hash).copied())
        }

        async fn revoke_refresh_token(&self, token_hash: &str) -> Result<(), anyhow::Error> {
            self.refresh_tokens.lock().unwrap().remove(token_hash);
            Ok(())
        }

        async fn delete_and_return_refresh_token(
            &self,
            token_hash: &str,
        ) -> Result<Option<(Uuid, chrono::DateTime<Utc>)>, anyhow::Error> {
            let uid = self.refresh_tokens.lock().unwrap().remove(token_hash);
            // Return some future expiry to pass the `expires_at > now()` check
            Ok(uid.map(|u| (u, chrono::Utc::now() + chrono::Duration::days(30))))
        }

        async fn revoke_all_refresh_tokens(&self, user_id: Uuid) -> Result<(), anyhow::Error> {
            let mut tokens = self.refresh_tokens.lock().unwrap();
            tokens.retain(|_, v| *v != user_id);
            Ok(())
        }

        async fn save_otp(
            &self,
            user_id: Uuid,
            otp_hash: &str,
            purpose: &str,
            _expires_at: chrono::DateTime<Utc>,
        ) -> Result<(), anyhow::Error> {
            let key = format!("{user_id}:{purpose}");
            let mut otps = self.otps.lock().unwrap();
            // Preserve attempts on overwrite (ON CONFLICT DO UPDATE — fix-phase2 C2)
            let attempts = otps.get(&key).map(|(_, a)| *a).unwrap_or(0);
            otps.insert(key, (otp_hash.to_owned(), attempts));
            Ok(())
        }

        async fn consume_otp(
            &self,
            user_id: Uuid,
            otp_hash: &str,
            purpose: &str,
        ) -> Result<bool, anyhow::Error> {
            let key = format!("{user_id}:{purpose}");
            let mut otps = self.otps.lock().unwrap();
            match otps.get(&key) {
                Some((stored, _)) if stored == otp_hash => {
                    otps.remove(&key);
                    Ok(true)
                }
                _ => Ok(false),
            }
        }

        async fn bump_otp_attempts(
            &self,
            user_id: Uuid,
            purpose: &str,
            max_attempts: i32,
        ) -> Result<i32, anyhow::Error> {
            let key = format!("{user_id}:{purpose}");
            let mut otps = self.otps.lock().unwrap();
            let entry = otps.remove(&key);
            match entry {
                Some((hash, attempts)) => {
                    let new_attempts = attempts + 1;
                    if new_attempts < max_attempts {
                        otps.insert(key, (hash, new_attempts));
                    }
                    Ok(new_attempts)
                }
                None => Ok(0),
            }
        }

        async fn set_status(
            &self,
            user_id: Uuid,
            status: AccountStatus,
        ) -> Result<(), anyhow::Error> {
            if let Some(u) = self.users.lock().unwrap().get_mut(&user_id) {
                u.status = status;
            }
            Ok(())
        }

        async fn get_status(&self, user_id: Uuid) -> Result<Option<AccountStatus>, anyhow::Error> {
            Ok(self.users.lock().unwrap().get(&user_id).map(|u| u.status))
        }

        async fn update_password(&self, user_id: Uuid, hash: &str) -> Result<(), anyhow::Error> {
            if let Some(u) = self.users.lock().unwrap().get_mut(&user_id) {
                u.password_hash = hash.to_owned();
            }
            Ok(())
        }

        async fn insert_suspension(
            &self,
            _user_id: Uuid,
            _is_permanent: bool,
            _reason: &str,
            _expires_at: Option<chrono::DateTime<Utc>>,
            _created_by: Uuid,
            _evidence_object_key: Option<&str>,
        ) -> Result<(), anyhow::Error> {
            Ok(())
        }

        async fn has_active_suspension(&self, _user_id: Uuid) -> Result<bool, anyhow::Error> {
            Ok(false)
        }

        async fn list_active_user_ids(&self) -> Result<Vec<Uuid>, anyhow::Error> {
            Ok(vec![])
        }

        async fn suspend_user_transactional(
            &self,
            user_id: Uuid,
            target: AccountStatus,
            _permanent: bool,
            _reason: &str,
            _expires_at: Option<chrono::DateTime<Utc>>,
            _admin_id: Uuid,
            _evidence_object_key: Option<&str>,
        ) -> Result<(), anyhow::Error> {
            self.set_status(user_id, target).await
        }

        async fn update_password_transactional(
            &self,
            user_id: Uuid,
            new_hash: &str,
        ) -> Result<(), anyhow::Error> {
            self.revoke_all_refresh_tokens(user_id).await?;
            self.update_password(user_id, new_hash).await?;
            Ok(())
        }

        async fn bump_otp_attempts_transactional(
            &self,
            user_id: Uuid,
            purpose: &str,
            max_attempts: i32,
        ) -> Result<i32, anyhow::Error> {
            self.bump_otp_attempts(user_id, purpose, max_attempts).await
        }

        async fn consume_otp_and_update_password_transactional(
            &self,
            user_id: Uuid,
            otp_hash: &str,
            purpose: &str,
            new_password_hash: &str,
        ) -> Result<bool, anyhow::Error> {
            let consumed = self.consume_otp(user_id, otp_hash, purpose).await?;
            if consumed {
                self.revoke_all_refresh_tokens(user_id).await?;
                self.update_password(user_id, new_password_hash).await?;
            }
            Ok(consumed)
        }
    }

    // --- AuthService integration tests with mock traits ---
    // DIP refactor complete: AuthService now uses Arc<dyn TokenIssuer> + Arc<dyn RateLimiter>,
    // enabling full unit testing of business logic without JWT keys or Redis.

    struct MockTokenIssuer;
    impl TokenIssuer for MockTokenIssuer {
        fn issue_access_token(
            &self,
            user_id: Uuid,
            email: &str,
            _status: AccountStatus,
            _role: Role,
        ) -> anyhow::Result<String> {
            Ok(format!("mock_token:{user_id}:{email}"))
        }
    }

    struct MockRateLimiter;
    #[async_trait::async_trait]
    impl RateLimiter for MockRateLimiter {
        async fn allow(&self, _purpose: &str, _user_key: &str) -> bool {
            true
        }
        async fn allow_raw(&self, _key: &str, _max_requests: i64, _window_secs: i64) -> bool {
            true
        }
    }

    fn test_auth_service() -> AuthService<MockAuthRepository> {
        AuthService {
            repo: Arc::new(MockAuthRepository::new()),
            token_issuer: Arc::new(MockTokenIssuer),
            refresh_ttl: 2_592_000,
            rate_limiter: Arc::new(MockRateLimiter),
            notifier: None,
        }
    }

    fn hash_test_password(pw: &str) -> String {
        bcrypt::hash(pw, bcrypt::DEFAULT_COST).unwrap()
    }

    #[tokio::test]
    async fn login_success_with_valid_password() {
        let svc = test_auth_service();
        let pw_hash = hash_test_password("Strong1!");
        svc.repo.insert_user(AuthUser {
            id: Uuid::now_v7(),
            email: "test@rejki.id".into(),
            password_hash: pw_hash,
            status: AccountStatus::Active,
            role: Role::User,
            phone: None,
            tos_accepted_at: Some(chrono::Utc::now()),
            tos_version: Some("v1".into()),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        });
        let result = svc
            .login(LoginInput {
                email: "test@rejki.id".into(),
                password: "Strong1!".into(),
            })
            .await;
        assert!(result.is_ok(), "Login harus sukses: {:?}", result.err());
        let tokens = result.unwrap();
        assert!(tokens.access_token.contains("mock_token:"));
        assert_eq!(tokens.token_type, "Bearer");
    }

    #[tokio::test]
    async fn login_fails_with_wrong_password() {
        let svc = test_auth_service();
        svc.repo.insert_user(AuthUser {
            id: Uuid::now_v7(),
            email: "test@rejki.id".into(),
            password_hash: hash_test_password("Strong1!"),
            status: AccountStatus::Active,
            role: Role::User,
            phone: None,
            tos_accepted_at: Some(chrono::Utc::now()),
            tos_version: Some("v1".into()),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        });
        let result = svc
            .login(LoginInput {
                email: "test@rejki.id".into(),
                password: "WrongPass1!".into(),
            })
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn login_fails_for_pending_verification() {
        let svc = test_auth_service();
        svc.repo.insert_user(AuthUser {
            id: Uuid::now_v7(),
            email: "pending@rejki.id".into(),
            password_hash: hash_test_password("Strong1!"),
            status: AccountStatus::PendingVerification,
            role: Role::User,
            phone: None,
            tos_accepted_at: Some(chrono::Utc::now()),
            tos_version: Some("v1".into()),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        });
        let result = svc
            .login(LoginInput {
                email: "pending@rejki.id".into(),
                password: "Strong1!".into(),
            })
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn login_fails_for_suspended_permanent() {
        let svc = test_auth_service();
        svc.repo.insert_user(AuthUser {
            id: Uuid::now_v7(),
            email: "banned@rejki.id".into(),
            password_hash: hash_test_password("Strong1!"),
            status: AccountStatus::SuspendedPermanent,
            role: Role::User,
            phone: None,
            tos_accepted_at: Some(chrono::Utc::now()),
            tos_version: Some("v1".into()),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        });
        let result = svc
            .login(LoginInput {
                email: "banned@rejki.id".into(),
                password: "Strong1!".into(),
            })
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn refresh_success_with_valid_token() {
        let svc = test_auth_service();
        let user = AuthUser {
            id: Uuid::now_v7(),
            email: "refresh@rejki.id".into(),
            password_hash: "any".into(),
            status: AccountStatus::Active,
            role: Role::User,
            phone: None,
            tos_accepted_at: Some(chrono::Utc::now()),
            tos_version: Some("v1".into()),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        svc.repo.insert_user(user.clone());
        // Pre-save a refresh token
        let token = generate_refresh_token();
        let token_hash = sha256_hex(&token);
        svc.repo
            .save_refresh_token(
                user.id,
                &token_hash,
                chrono::Utc::now() + chrono::Duration::days(30),
            )
            .await
            .unwrap();

        let result = svc
            .refresh(RefreshInput {
                refresh_token: token,
            })
            .await;
        assert!(result.is_ok(), "Refresh harus sukses: {:?}", result.err());
    }

    #[tokio::test]
    async fn refresh_fails_with_invalid_token() {
        let svc = test_auth_service();
        let result = svc
            .refresh(RefreshInput {
                refresh_token: "invalid_token".into(),
            })
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn logout_revokes_token() {
        let svc = test_auth_service();
        let token = generate_refresh_token();
        let token_hash = sha256_hex(&token);
        svc.repo
            .save_refresh_token(
                Uuid::now_v7(),
                &token_hash,
                chrono::Utc::now() + chrono::Duration::days(30),
            )
            .await
            .unwrap();
        assert!(svc.logout(&token).await.is_ok());
    }

    #[tokio::test]
    async fn suspend_one_rejects_pending_verification() {
        let svc = test_auth_service();
        let user = AuthUser {
            id: Uuid::now_v7(),
            email: "suspend@rejki.id".into(),
            password_hash: "hash".into(),
            status: AccountStatus::PendingVerification,
            role: Role::User,
            phone: None,
            tos_accepted_at: Some(chrono::Utc::now()),
            tos_version: Some("v1".into()),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        svc.repo.insert_user(user.clone());
        let result = svc
            .suspend_account(SuspendAccountParams {
                user_id: user.id,
                permanent: true,
                reason: "test",
                expires_at: None,
                admin_id: Uuid::now_v7(),
                evidence_object_key: Some("evidence_key"),
                notifier: None,
            })
            .await;
        assert!(
            result.is_err(),
            "Suspend PendingVerification seharusnya gagal"
        );
    }

    #[tokio::test]
    async fn suspend_one_success_for_active_user() {
        let svc = test_auth_service();
        let user = AuthUser {
            id: Uuid::now_v7(),
            email: "active-suspend@rejki.id".into(),
            password_hash: "hash".into(),
            status: AccountStatus::Active,
            role: Role::User,
            phone: None,
            tos_accepted_at: Some(chrono::Utc::now()),
            tos_version: Some("v1".into()),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        svc.repo.insert_user(user.clone());
        let result = svc
            .suspend_account(SuspendAccountParams {
                user_id: user.id,
                permanent: true,
                reason: "TOS violation",
                expires_at: None,
                admin_id: Uuid::now_v7(),
                evidence_object_key: Some("evidence_key"),
                notifier: None,
            })
            .await;
        assert!(
            result.is_ok(),
            "Suspend Active seharusnya sukses: {:?}",
            result.err()
        );
        // Verify status changed
        let new_status = svc.repo.get_status(user.id).await.unwrap().unwrap();
        assert_eq!(new_status, AccountStatus::SuspendedPermanent);
    }

    #[test]
    fn sha256_hex_produces_consistent_hash() {
        let h1 = sha256_hex("hello");
        let h2 = sha256_hex("hello");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64); // SHA-256 = 64 hex chars
        assert_ne!(sha256_hex("hello"), sha256_hex("world"));
    }

    #[test]
    fn generate_otp_is_6_digit_zero_padded() {
        // Run multiple times; all outputs must be 6 chars, all digits
        for _ in 0..50 {
            let otp = generate_otp();
            assert_eq!(otp.len(), 6, "OTP length not 6: {otp}");
            assert!(
                otp.chars().all(|c| c.is_ascii_digit()),
                "OTP not all digits: {otp}"
            );
        }
    }

    #[test]
    fn generate_refresh_token_is_64_hex_chars() {
        let token = generate_refresh_token();
        assert_eq!(token.len(), 64);
        assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
    }

    // --- State machine tests ---

    #[test]
    fn state_machine_active_to_suspended_temp() {
        assert!(AccountStatus::Active.can_transition_to(AccountStatus::SuspendedTemp));
    }

    #[test]
    fn state_machine_active_to_suspended_permanent() {
        assert!(AccountStatus::Active.can_transition_to(AccountStatus::SuspendedPermanent));
    }

    #[test]
    fn state_machine_suspended_temp_to_active() {
        assert!(AccountStatus::SuspendedTemp.can_transition_to(AccountStatus::Active));
    }

    #[test]
    fn state_machine_pending_verification_to_suspended_rejected() {
        assert!(!AccountStatus::PendingVerification
            .can_transition_to(AccountStatus::SuspendedPermanent));
        assert!(!AccountStatus::PendingVerification.can_transition_to(AccountStatus::SuspendedTemp));
    }

    #[test]
    fn state_machine_profile_incomplete_to_suspended_rejected() {
        assert!(
            !AccountStatus::ProfileIncomplete.can_transition_to(AccountStatus::SuspendedPermanent)
        );
        assert!(!AccountStatus::ProfileIncomplete.can_transition_to(AccountStatus::SuspendedTemp));
    }

    #[test]
    fn state_machine_pending_kyc_to_active() {
        assert!(AccountStatus::PendingKyc.can_transition_to(AccountStatus::Active));
    }

    #[test]
    fn state_machine_pending_kyc_to_rejected() {
        assert!(AccountStatus::PendingKyc.can_transition_to(AccountStatus::Rejected));
    }

    #[test]
    fn state_machine_rejected_to_pending_kyc() {
        assert!(AccountStatus::Rejected.can_transition_to(AccountStatus::PendingKyc));
    }

    #[test]
    fn state_machine_pending_verification_to_profile_incomplete() {
        assert!(
            AccountStatus::PendingVerification.can_transition_to(AccountStatus::ProfileIncomplete)
        );
    }

    #[test]
    fn state_machine_idempotent() {
        assert!(AccountStatus::Active.can_transition_to(AccountStatus::Active));
        assert!(AccountStatus::PendingVerification
            .can_transition_to(AccountStatus::PendingVerification));
        assert!(
            AccountStatus::SuspendedPermanent.can_transition_to(AccountStatus::SuspendedPermanent)
        );
    }

    // --- OTP purpose tests ---

    #[test]
    fn otp_purpose_as_str() {
        assert_eq!(OtpPurpose::Register.as_str(), "register");
        assert_eq!(OtpPurpose::ResetPassword.as_str(), "reset_password");
        assert_eq!(OtpPurpose::ChangePassword.as_str(), "change_password");
    }

    // --- Named constants ---

    #[test]
    fn constants_are_reasonable() {
        assert!(OTP_TTL_MINUTES > 0);
        assert!(MAX_OTP_ATTEMPTS > 0);
        assert!(MAX_OTP_ATTEMPTS < 10); // reasonable limit
        assert!(ACCESS_TOKEN_EXPIRY_SECS > 0);
        assert!(DEFAULT_REFRESH_TTL_SECS > ACCESS_TOKEN_EXPIRY_SECS as i64);
        assert!(LOGIN_RATE_LIMIT_MAX > 0);
        assert!(LOGIN_RATE_LIMIT_WINDOW_SECS > 0);
    }

    // ── Register tests ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn register_creates_user_and_dispatches_otp() {
        let svc = test_auth_service();
        let input = RegisterInput {
            email: "new@rejki.id".into(),
            password: "Strong1!".into(),
            phone: None,
            tos_accepted: true,
            tos_version: Some("v1".into()),
        };
        let result = svc.register(input).await;
        assert!(result.is_ok(), "Register harus sukses: {:?}", result.err());
        // User must exist after registration
        let user = svc.repo.find_by_email("new@rejki.id").await.unwrap();
        assert!(user.is_some());
    }

    #[tokio::test]
    async fn register_given_existing_email_when_pending_verification_then_resends_otp() {
        let svc = test_auth_service();
        svc.repo.insert_user(AuthUser {
            id: Uuid::now_v7(),
            email: "pending@rejki.id".into(),
            password_hash: hash_test_password("Strong1!"),
            status: AccountStatus::PendingVerification,
            role: Role::User,
            phone: None,
            tos_accepted_at: Some(chrono::Utc::now()),
            tos_version: Some("v1".into()),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        });
        let input = RegisterInput {
            email: "pending@rejki.id".into(),
            password: "Different1!".into(),
            phone: None,
            tos_accepted: true,
            tos_version: Some("v1".into()),
        };
        // Anti-enumeration: returns Ok even though user exists (no leak)
        let result = svc.register(input).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn register_given_active_email_when_register_then_silently_ok() {
        let svc = test_auth_service();
        svc.repo.insert_user(AuthUser {
            id: Uuid::now_v7(),
            email: "active@rejki.id".into(),
            password_hash: hash_test_password("Strong1!"),
            status: AccountStatus::Active,
            role: Role::User,
            phone: None,
            tos_accepted_at: Some(chrono::Utc::now()),
            tos_version: Some("v1".into()),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        });
        let input = RegisterInput {
            email: "active@rejki.id".into(),
            password: "Strong1!".into(),
            phone: None,
            tos_accepted: true,
            tos_version: Some("v1".into()),
        };
        // Anti-enumeration: returns Ok silently (no information leak)
        let result = svc.register(input).await;
        assert!(result.is_ok());
    }

    // ── Admin login tests ───────────────────────────────────────────────────

    #[tokio::test]
    async fn admin_login_success_with_valid_credentials() {
        let svc = test_auth_service();
        let pw_hash = hash_test_password("Admin1!");
        svc.repo.insert_user(AuthUser {
            id: Uuid::now_v7(),
            email: "admin@rejki.id".into(),
            password_hash: pw_hash,
            status: AccountStatus::Active,
            role: Role::Admin,
            phone: None,
            tos_accepted_at: Some(chrono::Utc::now()),
            tos_version: Some("v1".into()),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        });
        let result = svc
            .admin_login(AdminLoginInput {
                email: "admin@rejki.id".into(),
                password: "Admin1!".into(),
            })
            .await;
        assert!(
            result.is_ok(),
            "Admin login harus sukses: {:?}",
            result.err()
        );
        let tokens = result.unwrap();
        assert!(tokens.access_token.contains("mock_token:"));
        assert_eq!(tokens.token_type, "Bearer");
    }

    #[tokio::test]
    async fn admin_login_given_user_role_when_admin_login_then_unauthorized() {
        let svc = test_auth_service();
        svc.repo.insert_user(AuthUser {
            id: Uuid::now_v7(),
            email: "user@rejki.id".into(),
            password_hash: hash_test_password("Strong1!"),
            status: AccountStatus::Active,
            role: Role::User, // NOT admin
            phone: None,
            tos_accepted_at: Some(chrono::Utc::now()),
            tos_version: Some("v1".into()),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        });
        let result = svc
            .admin_login(AdminLoginInput {
                email: "user@rejki.id".into(),
                password: "Strong1!".into(),
            })
            .await;
        assert!(result.is_err(), "User non-admin tidak boleh login admin");
        match result.unwrap_err() {
            ServiceError::Unauthorized(_) => {} // expected — no leak
            other => panic!("Expected Unauthorized, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn admin_login_given_wrong_password_when_admin_login_then_unauthorized() {
        let svc = test_auth_service();
        svc.repo.insert_user(AuthUser {
            id: Uuid::now_v7(),
            email: "admin@rejki.id".into(),
            password_hash: hash_test_password("Admin1!"),
            status: AccountStatus::Active,
            role: Role::Admin,
            phone: None,
            tos_accepted_at: Some(chrono::Utc::now()),
            tos_version: Some("v1".into()),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        });
        let result = svc
            .admin_login(AdminLoginInput {
                email: "admin@rejki.id".into(),
                password: "WrongPass1!".into(),
            })
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn admin_login_given_nonexistent_email_when_admin_login_then_unauthorized() {
        let svc = test_auth_service();
        // Timing-safe: bcrypt verify against dummy hash, then fail
        let result = svc
            .admin_login(AdminLoginInput {
                email: "ghost@rejki.id".into(),
                password: "Whatever1!".into(),
            })
            .await;
        assert!(
            result.is_err(),
            "Non-existent admin login should fail generically"
        );
    }

    // ── Verify OTP tests ────────────────────────────────────────────────────

    #[tokio::test]
    async fn verify_otp_given_valid_otp_when_verify_then_consumed() {
        let svc = test_auth_service();
        let user_id = Uuid::now_v7();
        svc.repo.insert_user(AuthUser {
            id: user_id,
            email: "otp-valid@rejki.id".into(),
            password_hash: hash_test_password("Strong1!"),
            status: AccountStatus::PendingVerification,
            role: Role::User,
            phone: None,
            tos_accepted_at: Some(chrono::Utc::now()),
            tos_version: Some("v1".into()),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        });
        // Pre-save a known OTP hash
        let otp_value = "123456";
        let otp_hash = sha256_hex(otp_value);
        svc.repo
            .save_otp(
                user_id,
                &otp_hash,
                OtpPurpose::Register.as_str(),
                chrono::Utc::now() + chrono::Duration::minutes(5),
            )
            .await
            .unwrap();

        let result = svc
            .verify_otp(VerifyOtpInput {
                email: "otp-valid@rejki.id".into(),
                otp: otp_value.into(),
                purpose: "register".into(),
            })
            .await;
        assert!(
            result.is_ok(),
            "Verify valid OTP harus sukses: {:?}",
            result.err()
        );
    }

    #[tokio::test]
    async fn verify_otp_given_wrong_otp_when_verify_then_increments_attempts() {
        common_tracing::init_tracing();
        let svc = test_auth_service();
        let user_id = Uuid::now_v7();
        svc.repo.insert_user(AuthUser {
            id: user_id,
            email: "otp-wrong@rejki.id".into(),
            password_hash: hash_test_password("Strong1!"),
            status: AccountStatus::PendingVerification,
            role: Role::User,
            phone: None,
            tos_accepted_at: Some(chrono::Utc::now()),
            tos_version: Some("v1".into()),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        });
        let otp_hash = sha256_hex("999999");
        svc.repo
            .save_otp(
                user_id,
                &otp_hash,
                OtpPurpose::Register.as_str(),
                chrono::Utc::now() + chrono::Duration::minutes(5),
            )
            .await
            .unwrap();

        let result = svc
            .verify_otp(VerifyOtpInput {
                email: "otp-wrong@rejki.id".into(),
                otp: "111111".into(),
                purpose: "register".into(),
            })
            .await;
        // Wrong OTP: should return error (not just Ok)
        assert!(result.is_err(), "Wrong OTP harus gagal");
    }

    // ── Rate limit tests ────────────────────────────────────────────────────

    fn test_rate_limited_service() -> AuthService<MockAuthRepository> {
        AuthService {
            repo: Arc::new(MockAuthRepository::new()),
            token_issuer: Arc::new(MockTokenIssuer),
            refresh_ttl: 2_592_000,
            rate_limiter: Arc::new(DenyingRateLimiter),
            notifier: None,
        }
    }

    /// A rate limiter that always denies — simulates brute-force triggered state.
    struct DenyingRateLimiter;
    #[async_trait::async_trait]
    impl RateLimiter for DenyingRateLimiter {
        async fn allow(&self, _purpose: &str, _user_key: &str) -> bool {
            false
        }
        async fn allow_raw(&self, _key: &str, _max_requests: i64, _window_secs: i64) -> bool {
            false
        }
    }

    #[tokio::test]
    async fn login_given_rate_limit_exceeded_when_login_then_too_many_requests() {
        let svc = test_rate_limited_service();
        svc.repo.insert_user(AuthUser {
            id: Uuid::now_v7(),
            email: "ratelimited@rejki.id".into(),
            password_hash: hash_test_password("Strong1!"),
            status: AccountStatus::Active,
            role: Role::User,
            phone: None,
            tos_accepted_at: Some(chrono::Utc::now()),
            tos_version: Some("v1".into()),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        });
        let result = svc
            .login(LoginInput {
                email: "ratelimited@rejki.id".into(),
                password: "Strong1!".into(),
            })
            .await;
        assert!(result.is_err());
        // Must be RateLimited, not Unauthorized
        match result.unwrap_err() {
            ServiceError::RateLimited(_) => {} // expected
            other => panic!("Expected RateLimited, got {:?}", other),
        }
    }

    // ── Resend OTP tests ────────────────────────────────────────────────────

    #[tokio::test]
    async fn resend_otp_given_existing_otp_when_resend_then_does_not_reset_attempts() {
        let svc = test_auth_service();
        let user_id = Uuid::now_v7();
        svc.repo.insert_user(AuthUser {
            id: user_id,
            email: "resend@rejki.id".into(),
            password_hash: hash_test_password("Strong1!"),
            status: AccountStatus::PendingVerification,
            role: Role::User,
            phone: None,
            tos_accepted_at: Some(chrono::Utc::now()),
            tos_version: Some("v1".into()),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        });
        // Pre-save an OTP with attempts
        let otp_hash = sha256_hex("111111");
        svc.repo
            .save_otp(
                user_id,
                &otp_hash,
                OtpPurpose::Register.as_str(),
                chrono::Utc::now() + chrono::Duration::minutes(5),
            )
            .await
            .unwrap();
        // Bump attempts to 2
        svc.repo
            .bump_otp_attempts_transactional(user_id, OtpPurpose::Register.as_str(), 5)
            .await
            .unwrap();
        svc.repo
            .bump_otp_attempts_transactional(user_id, OtpPurpose::Register.as_str(), 5)
            .await
            .unwrap();

        let result = svc
            .resend_otp(ResendOtpInput {
                email: "resend@rejki.id".into(),
                purpose: "register".into(),
            })
            .await;
        assert!(result.is_ok(), "Resend harus sukses: {:?}", result.err());
        // Attempts MUST NOT be reset (fix-phase2 C2)
        let otps = svc.repo.otps.lock().unwrap();
        let key = format!("{user_id}:register");
        if let Some((_, attempts)) = otps.get(&key) {
            assert!(
                *attempts >= 2,
                "Attempts tetap >= 2 setelah resend: got {}",
                attempts
            );
        }
    }

    // --- Handler integration-type tests (mock repo via AuthService) ---
    // Full AuthService tests are blocked by concrete JwtService/OtpRateLimiter types.
    // These will be fully testable after the DIP refactor (tasks 9.1-9.7).
    // See tests/handlers.rs and tests/integration.rs for covering handlers.

    #[test]
    fn account_status_as_str_roundtrip() {
        let variants = [
            AccountStatus::PendingVerification,
            AccountStatus::ProfileIncomplete,
            AccountStatus::PendingKyc,
            AccountStatus::Rejected,
            AccountStatus::Active,
            AccountStatus::SuspendedTemp,
            AccountStatus::SuspendedPermanent,
        ];
        for v in &variants {
            let s = v.as_str();
            let parsed: AccountStatus = s.parse().unwrap();
            assert_eq!(*v, parsed);
        }
    }

    #[test]
    fn role_as_str_roundtrip() {
        assert_eq!(Role::User.as_str(), "user");
        assert_eq!(Role::Admin.as_str(), "admin");
        assert_eq!("user".parse::<Role>().unwrap(), Role::User);
        assert_eq!("admin".parse::<Role>().unwrap(), Role::Admin);
        assert!("super_admin".parse::<Role>().is_err());
    }
}
