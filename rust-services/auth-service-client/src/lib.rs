use uuid::Uuid;

/// Peran pengguna (multi-tier RBAC). Pure hierarchy dengan rank numerik.
/// Role yang lebih tinggi bisa mengakses endpoint role yang sama/lebih rendah.
/// Ref: openspec/changes/ws-multi-tier-rbac, D1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Role {
    /// Pengguna biasa — default. Rank 20.
    #[default]
    User,
    /// Pengguna yang sudah terverifikasi — akses premium (chat, post iklan). Rank 40.
    UserVerified,
    /// Moderator — akses report & corporate-comms. Rank 60.
    Moderator,
    /// Admin iklan — akses admin endpoint iklan + user. Rank 80.
    AdminIklan,
    /// Admin user — akses admin endpoint user/KYC + suspend. Rank 80.
    AdminUser,
    /// Super admin — akses semua endpoint. Rank 100.
    SuperAdmin,
}

impl Role {
    /// Hierarchy rank — semakin tinggi semakin besar akses.
    pub fn rank(&self) -> u8 {
        match self {
            Role::User => 20,
            Role::UserVerified => 40,
            Role::Moderator => 60,
            Role::AdminIklan => 80,
            Role::AdminUser => 80,
            Role::SuperAdmin => 100,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Role::User => "user",
            Role::UserVerified => "user_verified",
            Role::Moderator => "moderator",
            Role::AdminIklan => "admin_iklan",
            Role::AdminUser => "admin_user",
            Role::SuperAdmin => "super_admin",
        }
    }

    /// Apakah role ini memiliki akses admin (rank >= moderator/60).
    pub fn is_admin(&self) -> bool {
        self.rank() >= 60
    }
}

impl std::str::FromStr for Role {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "user" => Ok(Role::User),
            "user_verified" => Ok(Role::UserVerified),
            "moderator" => Ok(Role::Moderator),
            "admin_iklan" => Ok(Role::AdminIklan),
            "admin_user" => Ok(Role::AdminUser),
            "super_admin" => Ok(Role::SuperAdmin),
            _ => Err(()),
        }
    }
}

/// Status akun (state machine onboarding). Tipe publik agar dipakai lintas domain
/// (auth-service sebagai pemilik, user-service/admin sebagai pemanggil via AuthClient).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountStatus {
    PendingVerification,
    ProfileIncomplete,
    PendingKyc,
    Rejected,
    Active,
    SuspendedTemp,
    SuspendedPermanent,
}

impl AccountStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            AccountStatus::PendingVerification => "pending_verification",
            AccountStatus::ProfileIncomplete => "profile_incomplete",
            AccountStatus::PendingKyc => "pending_kyc",
            AccountStatus::Rejected => "rejected",
            AccountStatus::Active => "active",
            AccountStatus::SuspendedTemp => "suspended_temp",
            AccountStatus::SuspendedPermanent => "suspended_permanent",
        }
    }

    /// Hanya akun aktif yang boleh memakai fitur (gating).
    pub fn can_use_features(&self) -> bool {
        matches!(self, AccountStatus::Active)
    }

    /// Apakah transisi dari status ini ke `to` diizinkan oleh state machine.
    pub fn can_transition_to(&self, to: AccountStatus) -> bool {
        use AccountStatus::*;
        match (self, to) {
            (PendingVerification, ProfileIncomplete) => true,
            (ProfileIncomplete, PendingKyc) => true,
            (PendingKyc, Active) => true,
            (PendingKyc, Rejected) => true,
            (Rejected, PendingKyc) => true,
            (Active, SuspendedTemp) => true,
            (Active, SuspendedPermanent) => true,
            (SuspendedTemp, Active) => true,
            (SuspendedTemp, SuspendedPermanent) => true,
            (a, b) if *a == b => true, // idempoten
            _ => false,
        }
    }
}

impl std::str::FromStr for AccountStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending_verification" => Ok(AccountStatus::PendingVerification),
            "profile_incomplete" => Ok(AccountStatus::ProfileIncomplete),
            "pending_kyc" => Ok(AccountStatus::PendingKyc),
            "rejected" => Ok(AccountStatus::Rejected),
            "active" => Ok(AccountStatus::Active),
            "suspended_temp" => Ok(AccountStatus::SuspendedTemp),
            "suspended_permanent" => Ok(AccountStatus::SuspendedPermanent),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuthClaims {
    pub user_id: Uuid,
    pub email: String,
    /// Status akun saat token diterbitkan (untuk gating cepat). Opsional demi
    /// kompatibilitas dengan token lama yang belum memuat status.
    #[serde(default)]
    pub status: Option<AccountStatus>,
    /// Peran pengguna saat token diterbitkan. Opsional demi kompatibilitas dengan
    /// token lama yang belum memuat role — default menjadi `user` (default-deny admin).
    #[serde(default)]
    pub role: Option<Role>,
}

#[async_trait::async_trait]
pub trait AuthClient: Send + Sync {
    async fn validate_token(&self, jwt: &str) -> Result<AuthClaims, AuthClientError>;

    /// Ambil status akun terkini (sumber kebenaran, bukan dari klaim token).
    async fn get_account_status(&self, user_id: Uuid) -> Result<AccountStatus, AuthClientError>;

    /// Ambil alamat email akun. Dipakai user-service untuk mengirim email hasil KYC
    /// (K12) ketika pemanggil adalah admin (claims berisi email admin, bukan user).
    /// Arah: user/admin -> auth (auth pemilik data akun, D2).
    async fn get_account_email(&self, user_id: Uuid) -> Result<String, AuthClientError>;

    /// Ubah status akun (transisi divalidasi oleh auth-service). Dipanggil oleh
    /// user-service/admin saat KYC approve/reject & suspend. Arah: user/admin -> auth.
    async fn set_account_status(
        &self,
        user_id: Uuid,
        status: AccountStatus,
    ) -> Result<(), AuthClientError>;

    /// Ambil semua user_id pengguna aktif — dipakai untuk broadcast notifikasi
    /// (mis. corporate-comms service menyiarkan artikel ke seluruh pengguna).
    /// Arah: corporate-comms -> auth.
    async fn list_active_user_ids(&self) -> Result<Vec<Uuid>, AuthClientError>;
}

#[derive(Debug, thiserror::Error)]
pub enum AuthClientError {
    #[error("token invalid or expired")]
    InvalidToken,
    #[error("auth service unavailable")]
    Unavailable,
    #[error("user not found")]
    NotFound,
    #[error("invalid status transition")]
    InvalidTransition,
}
