use uuid::Uuid;

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
}

#[async_trait::async_trait]
pub trait AuthClient: Send + Sync {
    async fn validate_token(&self, jwt: &str) -> Result<AuthClaims, AuthClientError>;

    /// Ambil status akun terkini (sumber kebenaran, bukan dari klaim token).
    async fn get_account_status(&self, user_id: Uuid) -> Result<AccountStatus, AuthClientError>;

    /// Ubah status akun (transisi divalidasi oleh auth-service). Dipanggil oleh
    /// user-service/admin saat KYC approve/reject & suspend. Arah: user/admin -> auth.
    async fn set_account_status(
        &self,
        user_id: Uuid,
        status: AccountStatus,
    ) -> Result<(), AuthClientError>;
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
