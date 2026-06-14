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
use crate::domain::repository::AuthRepository;
use crate::infrastructure::JwtService;

use crate::infrastructure::OtpRateLimiter;
use notification_service_client::{EmailMessage, NotificationClient};

/// TTL OTP dalam menit (RQ2 — default usulan).
const OTP_TTL_MINUTES: i64 = 5;
/// Maks percobaan verifikasi OTP sebelum OTP dibatalkan (RQ2).
const MAX_OTP_ATTEMPTS: i32 = 5;
/// Durasi access token (detik) — digunakan sebagai `expires_in` pada `TokenPair`.
const ACCESS_TOKEN_EXPIRY_SECS: u64 = 900;
/// Versi T&C default bila user tidak menyertakan.
const DEFAULT_TOS_VERSION: &str = "v1";
/// TTL refresh token default (30 hari) — dipakai sebagai fallback di router.
pub const DEFAULT_REFRESH_TTL_SECS: i64 = 2_592_000;
/// TTL access token default (15 menit) — dipakai sebagai fallback di router.
pub const DEFAULT_ACCESS_TTL_SECS: i64 = 900;
/// Kategori storage untuk bukti penangguhan.
pub mod storage_category {
    pub const SUSPENSION_EVIDENCE: &str = "suspension-evidence";
}

pub struct AuthService<R: AuthRepository> {
    repo: Arc<R>,
    jwt: Arc<JwtService>,
    refresh_ttl: i64, // detik
    rate_limit: OtpRateLimiter,
    /// Kontrak pengiriman notifikasi (email OTP). In-process di monolith;
    /// di-inject dari Composition Root. Opsional agar standalone/test tetap jalan.
    notifier: Option<Arc<dyn NotificationClient>>,
}

impl<R: AuthRepository> AuthService<R> {
    pub fn new(repo: Arc<R>, jwt: Arc<JwtService>, refresh_ttl: i64) -> Self {
        Self {
            repo,
            jwt,
            refresh_ttl,
            rate_limit: OtpRateLimiter::from_env(),
            notifier: None,
        }
    }

    /// Inject NotificationClient (dipanggil dari Composition Root). Builder agar
    /// `new()` lama tetap kompatibel untuk standalone/test.
    pub fn with_notifier(mut self, notifier: Arc<dyn NotificationClient>) -> Self {
        self.notifier = Some(notifier);
        self
    }

    /// Kirim OTP ke email lewat kontrak NotificationClient (domain notification yang
    /// memegang detail Redis/SMTP). Bila notifier tidak di-inject atau gagal, log
    /// peringatan tanpa membocorkan OTP dan tanpa menggagalkan alur utama.
    async fn dispatch_otp_email(&self, to: &str, purpose: &str, otp: &str) {
        let Some(notifier) = &self.notifier else {
            tracing::warn!(
                to,
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
            tracing::warn!(error = %e, to, purpose, "gagal kirim email OTP via notifier");
        }
    }

    pub async fn register(&self, input: RegisterInput) -> Result<(), anyhow::Error> {
        // Catatan: persetujuan T&C & kekuatan password sudah divalidasi di DTO (ValidatedJson → 422).

        // Anti-enumeration: jika email sudah ada, jangan bocorkan. Untuk akun yang masih
        // pending_verification, kirim ulang OTP; selain itu balas sukses generik tanpa aksi.
        if let Some(existing) = self.repo.find_by_email(&input.email).await? {
            if existing.status == AccountStatus::PendingVerification {
                let otp = generate_otp();
                let otp_hash = hash_otp(&otp);
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
        let otp_hash = hash_otp(&otp);
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

        let otp_hash = hash_otp(&input.otp);
        let consumed = self
            .repo
            .consume_otp(user.id, &otp_hash, &input.purpose)
            .await?;

        if !consumed {
            // Catat percobaan gagal; OTP dibatalkan bila melampaui batas (anti brute-force).
            self.repo
                .bump_otp_attempts(user.id, &input.purpose, MAX_OTP_ATTEMPTS)
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

    pub async fn login(&self, input: LoginInput) -> Result<TokenPair, anyhow::Error> {
        let user = self
            .repo
            .find_by_email(&input.email)
            .await?
            .ok_or_else(|| anyhow!("email atau password salah"))?;

        // Cek kelayakan akun via status (menggantikan is_verified).
        let mut effective_status = user.status;
        match user.status {
            AccountStatus::PendingVerification => {
                return Err(anyhow!("akun belum diverifikasi, cek email untuk OTP"));
            }
            AccountStatus::SuspendedPermanent => {
                return Err(anyhow!("akun sedang ditangguhkan"));
            }
            AccountStatus::SuspendedTemp => {
                // Auto-pulih: jika tidak ada penangguhan yang masih berlaku, kembalikan ke active.
                if self.repo.has_active_suspension(user.id).await? {
                    return Err(anyhow!("akun sedang ditangguhkan"));
                }
                self.repo.set_status(user.id, AccountStatus::Active).await?;
                effective_status = AccountStatus::Active;
            }
            _ => {}
        }

        if !verify(&input.password, &user.password_hash).context("gagal verifikasi password")? {
            return Err(anyhow!("email atau password salah"));
        }

        self.issue_tokens(&user, effective_status).await
    }

    /// Admin login: verifikasi email+password, cek `role=admin` + `status=active`.
    /// Anti-enumeration: pesan galat seragam untuk kredensial salah, non-admin,
    /// akun tidak ditemukan — tidak membocorkan keberadaan akun.
    /// Ref: openspec/changes/add-admin-rbac D3.
    pub async fn admin_login(&self, input: AdminLoginInput) -> Result<TokenPair, anyhow::Error> {
        let user = match self.repo.find_by_email(&input.email).await? {
            Some(u) => u,
            None => return Err(anyhow!("email atau password salah")),
        };

        // Verifikasi role: hanya admin yang boleh masuk.
        if !user.role.is_admin() {
            return Err(anyhow!("email atau password salah"));
        }

        // Cek status: hanya active yang boleh masuk.
        match user.status {
            AccountStatus::Active => {}
            AccountStatus::SuspendedPermanent | AccountStatus::SuspendedTemp => {
                return Err(anyhow!("email atau password salah"));
            }
            _ => return Err(anyhow!("email atau password salah")),
        }

        if !verify(&input.password, &user.password_hash).context("gagal verifikasi password")? {
            return Err(anyhow!("email atau password salah"));
        }

        // Audit: login admin tercatat.
        tracing::info!(
            user_id = %user.id,
            email = %user.email,
            "admin login berhasil"
        );

        self.issue_tokens(&user, user.status).await
    }

    /// Shared helper: issue JWT + refresh token + save ke DB.
    /// Dipakai oleh `login`, `admin_login`, dan `refresh`.
    async fn issue_tokens(
        &self,
        user: &crate::domain::entity::AuthUser,
        effective_status: crate::domain::entity::AccountStatus,
    ) -> Result<TokenPair, anyhow::Error> {
        let access_token =
            self.jwt
                .issue_access_token(user.id, &user.email, effective_status, user.role)?;
        let refresh_token = generate_refresh_token();
        let refresh_hash = hash_token(&refresh_token);
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
        let token_hash = hash_token(&input.refresh_token);

        let user_id = self
            .repo
            .find_user_by_refresh_token(&token_hash)
            .await?
            .ok_or_else(|| anyhow!("refresh token tidak valid atau sudah expired"))?;

        // Rotasi: hapus token lama, issue token baru
        self.repo.revoke_refresh_token(&token_hash).await?;

        let user = self
            .repo
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| anyhow!("user tidak ditemukan"))?;

        self.issue_tokens(&user, user.status).await
    }

    pub async fn logout(&self, refresh_token: &str) -> Result<(), anyhow::Error> {
        let token_hash = hash_token(refresh_token);
        self.repo.revoke_refresh_token(&token_hash).await
    }

    pub async fn resend_otp(&self, input: ResendOtpInput) -> Result<(), anyhow::Error> {
        if !self.rate_limit.allow(&input.purpose, &input.email).await {
            return Err(anyhow!("terlalu banyak permintaan OTP, coba lagi nanti"));
        }
        let user = self
            .repo
            .find_by_email(&input.email)
            .await?
            .ok_or_else(|| anyhow!("user tidak ditemukan"))?;

        let otp = generate_otp();
        let otp_hash = hash_otp(&otp);
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
        if !self
            .rate_limit
            .allow(OtpPurpose::ResetPassword.as_str(), email)
            .await
        {
            return Ok(());
        }
        if let Some(user) = self.repo.find_by_email(email).await? {
            let otp = generate_otp();
            let otp_hash = hash_otp(&otp);
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

    /// US-05: set password baru dengan OTP `reset_password`. Cabut SEMUA refresh token.
    pub async fn reset_password(&self, input: ResetPasswordInput) -> Result<(), anyhow::Error> {
        let user = self
            .repo
            .find_by_email(&input.email)
            .await?
            .ok_or_else(|| anyhow!("OTP tidak valid atau sudah expired"))?;

        let otp_hash = hash_otp(&input.otp);
        let consumed = self
            .repo
            .consume_otp(user.id, &otp_hash, OtpPurpose::ResetPassword.as_str())
            .await?;
        if !consumed {
            self.repo
                .bump_otp_attempts(
                    user.id,
                    OtpPurpose::ResetPassword.as_str(),
                    MAX_OTP_ATTEMPTS,
                )
                .await?;
            return Err(anyhow!("OTP tidak valid atau sudah expired"));
        }

        let new_hash = hash(&input.new_password, DEFAULT_COST).context("gagal hash password")?;
        self.repo.update_password(user.id, &new_hash).await?;
        self.repo.revoke_all_refresh_tokens(user.id).await?;
        Ok(())
    }

    /// US-06: ubah password oleh user terautentikasi dengan OTP `change_password`.
    /// Cabut SEMUA refresh token (sesi lain) demi keamanan.
    pub async fn change_password(
        &self,
        user_id: Uuid,
        input: ChangePasswordInput,
    ) -> Result<(), anyhow::Error> {
        let otp_hash = hash_otp(&input.otp);
        let consumed = self
            .repo
            .consume_otp(user_id, &otp_hash, OtpPurpose::ChangePassword.as_str())
            .await?;
        if !consumed {
            self.repo
                .bump_otp_attempts(
                    user_id,
                    OtpPurpose::ChangePassword.as_str(),
                    MAX_OTP_ATTEMPTS,
                )
                .await?;
            return Err(anyhow!("OTP tidak valid atau sudah expired"));
        }

        let new_hash = hash(&input.new_password, DEFAULT_COST).context("gagal hash password")?;
        self.repo.update_password(user_id, &new_hash).await?;
        self.repo.revoke_all_refresh_tokens(user_id).await?;
        Ok(())
    }

    /// Minta OTP `change_password` untuk user terautentikasi (dikirim ke email terdaftar).
    pub async fn request_change_password_otp(&self, user_id: Uuid) -> Result<(), anyhow::Error> {
        if !self
            .rate_limit
            .allow(OtpPurpose::ChangePassword.as_str(), &user_id.to_string())
            .await
        {
            return Err(anyhow!("terlalu banyak permintaan OTP, coba lagi nanti"));
        }
        let user = self
            .repo
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| anyhow!("user tidak ditemukan"))?;
        let otp = generate_otp();
        let otp_hash = hash_otp(&otp);
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
    /// Mengubah status, mencatat riwayat+audit, dan mencabut seluruh refresh token.
    pub async fn suspend_account(
        &self,
        user_id: Uuid,
        permanent: bool,
        reason: &str,
        expires_at: Option<chrono::DateTime<Utc>>,
        admin_id: Uuid,
        evidence_object_key: Option<&str>,
    ) -> Result<(), anyhow::Error> {
        let target = if permanent {
            AccountStatus::SuspendedPermanent
        } else {
            AccountStatus::SuspendedTemp
        };

        // Set status langsung (transisi suspend valid dari status apa pun yang aktif).
        self.repo.set_status(user_id, target).await?;
        self.repo
            .insert_suspension(
                user_id,
                permanent,
                reason,
                expires_at,
                admin_id,
                evidence_object_key,
            )
            .await?;
        // Cabut sesi agar token lama tidak bisa refresh.
        self.repo.revoke_all_refresh_tokens(user_id).await?;

        tracing::info!(
            user_id = %user_id, admin_id = %admin_id, permanent, reason, evidence_object_key,
            "account suspended"
        );
        Ok(())
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

fn hash_otp(otp: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(otp.as_bytes());
    format!("{:x}", h.finalize())
}

fn hash_token(token: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(token.as_bytes());
    format!("{:x}", h.finalize())
}
