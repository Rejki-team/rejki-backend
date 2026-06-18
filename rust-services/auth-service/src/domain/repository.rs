use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::entity::{AccountStatus, AuthUser};

// `async fn` di trait kini stabil; lint hanya menyoroti ketiadaan Send bound otomatis.
// Repository dipakai in-process; cukup di-allow.
#[allow(async_fn_in_trait)]
pub trait AuthRepository: Send + Sync {
    async fn find_by_email(&self, email: &str) -> Result<Option<AuthUser>, anyhow::Error>;

    async fn find_by_id(&self, id: Uuid) -> Result<Option<AuthUser>, anyhow::Error>;

    /// Buat user baru. `phone` sudah dalam bentuk terenkripsi (ciphertext) bila ada.
    async fn create_user(
        &self,
        email: &str,
        password_hash: &str,
        phone_encrypted: Option<&str>,
        tos_version: &str,
    ) -> Result<AuthUser, anyhow::Error>;

    async fn save_refresh_token(
        &self,
        user_id: Uuid,
        token_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), anyhow::Error>;

    /// Kembalikan user_id jika token valid dan belum expired.
    async fn find_user_by_refresh_token(
        &self,
        token_hash: &str,
    ) -> Result<Option<Uuid>, anyhow::Error>;

    async fn revoke_refresh_token(&self, token_hash: &str) -> Result<(), anyhow::Error>;

    /// Hapus refresh token secara atomik dan kembalikan user_id. Dipakai di refresh
    /// untuk menghindari race condition (DELETE-first pattern).
    async fn delete_and_return_refresh_token(
        &self,
        token_hash: &str,
    ) -> Result<Option<(Uuid, chrono::DateTime<Utc>)>, anyhow::Error>;

    /// Cabut SEMUA refresh token milik user (mis. saat reset password).
    async fn revoke_all_refresh_tokens(&self, user_id: Uuid) -> Result<(), anyhow::Error>;

    async fn save_otp(
        &self,
        user_id: Uuid,
        otp_hash: &str,
        purpose: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), anyhow::Error>;

    /// Cek OTP valid & belum expired, lalu hapus sekaligus (atomic — tidak ada race condition).
    async fn consume_otp(
        &self,
        user_id: Uuid,
        otp_hash: &str,
        purpose: &str,
    ) -> Result<bool, anyhow::Error>;

    /// Tambah counter percobaan gagal untuk OTP (user, purpose). Bila counter mencapai
    /// `max_attempts`, OTP dihapus agar tidak bisa di-brute-force lagi. Mengembalikan
    /// jumlah percobaan terkini (0 bila tidak ada OTP aktif).
    async fn bump_otp_attempts(
        &self,
        user_id: Uuid,
        purpose: &str,
        max_attempts: i32,
    ) -> Result<i32, anyhow::Error>;

    /// Ubah status akun (state machine divalidasi di application layer).
    async fn set_status(&self, user_id: Uuid, status: AccountStatus) -> Result<(), anyhow::Error>;

    /// Ambil status akun terkini.
    async fn get_status(&self, user_id: Uuid) -> Result<Option<AccountStatus>, anyhow::Error>;

    /// Perbarui password hash user.
    async fn update_password(
        &self,
        user_id: Uuid,
        password_hash: &str,
    ) -> Result<(), anyhow::Error>;

    /// Catat penangguhan akun (riwayat + audit). `expires_at` None = permanen.
    async fn insert_suspension(
        &self,
        user_id: Uuid,
        is_permanent: bool,
        reason: &str,
        expires_at: Option<DateTime<Utc>>,
        created_by: Uuid,
        evidence_object_key: Option<&str>,
    ) -> Result<(), anyhow::Error>;

    /// Apakah masih ada penangguhan yang berlaku untuk user (permanen, atau sementara
    /// yang `expires_at`-nya belum lewat). Dipakai login untuk auto-pulih suspend sementara.
    async fn has_active_suspension(&self, user_id: Uuid) -> Result<bool, anyhow::Error>;

    /// Ambil semua user_id pengguna aktif — dipakai untuk broadcast notifikasi
    /// (mis. corporate-comms service menyiarkan artikel ke seluruh pengguna).
    async fn list_active_user_ids(&self) -> Result<Vec<Uuid>, anyhow::Error>;

    /// Suspend user secara atomik: set_status + insert_suspension + revoke_all_refresh_tokens
    /// dalam satu transaction. Commit otomatis bila semua sukses; rollback bila gagal.
    #[allow(clippy::too_many_arguments)]
    async fn suspend_user_transactional(
        &self,
        user_id: Uuid,
        target: AccountStatus,
        permanent: bool,
        reason: &str,
        expires_at: Option<DateTime<Utc>>,
        admin_id: Uuid,
        evidence_object_key: Option<&str>,
    ) -> Result<(), anyhow::Error>;

    /// Update password + revoke_all_refresh_tokens dalam satu transaction.
    /// Token dicabut DULU sebelum password diperbarui (anti data-loss pada refresh token compromised).
    async fn update_password_transactional(
        &self,
        user_id: Uuid,
        new_hash: &str,
    ) -> Result<(), anyhow::Error>;

    /// Bump OTP attempts + conditional DELETE dalam satu transaction.
    /// Mencegah race condition di mana save_otp (resend) meng-upsert OTP baru
    /// yang langsung dihapus oleh DELETE yang masih in-flight.
    async fn bump_otp_attempts_transactional(
        &self,
        user_id: Uuid,
        purpose: &str,
        max_attempts: i32,
    ) -> Result<i32, anyhow::Error>;

    /// Consume OTP + revoke refresh tokens + update password dalam SATU transaction.
    /// Mencegah OTP terlanjur dikonsumsi walau password update gagal.
    async fn consume_otp_and_update_password_transactional(
        &self,
        user_id: Uuid,
        otp_hash: &str,
        purpose: &str,
        new_password_hash: &str,
    ) -> Result<bool, anyhow::Error>;
}
