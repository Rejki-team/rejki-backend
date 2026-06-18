use chrono::Utc;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use auth_service_client::{AuthClaims, AuthClientError, Role};

use crate::domain::entity::AccountStatus;
use crate::domain::token::TokenIssuer;
use crate::domain::token::TokenValidator;

/// Internal JWT claims structure (superset dari AuthClaims publik).
#[derive(Debug, Serialize, Deserialize)]
struct JwtClaims {
    sub: String, // user_id sebagai string
    email: String,
    status: String, // status akun (untuk gating cepat)
    #[serde(default)]
    role: Option<String>, // peran pengguna (RBAC admin, opsional untuk kompatibilitas token lama)
    iat: i64,
    exp: i64,
}

pub struct JwtService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    access_ttl: i64, // detik
}

impl JwtService {
    /// Buat JwtService dari path file PEM.
    pub fn from_files(
        private_pem: &str,
        public_pem: &str,
        access_ttl_secs: i64,
    ) -> anyhow::Result<Self> {
        let encoding_key = EncodingKey::from_rsa_pem(private_pem.as_bytes())
            .map_err(|e| anyhow::anyhow!("gagal load private key: {e}"))?;
        let decoding_key = DecodingKey::from_rsa_pem(public_pem.as_bytes())
            .map_err(|e| anyhow::anyhow!("gagal load public key: {e}"))?;
        Ok(Self {
            encoding_key,
            decoding_key,
            access_ttl: access_ttl_secs,
        })
    }

    /// Validasi token dan kembalikan claims. Dipakai oleh AuthInProcessClient.
    /// Delegasi ke TokenValidator trait implementation.
    pub fn validate_token(&self, token: &str) -> Result<AuthClaims, AuthClientError> {
        TokenValidator::validate_token(self, token)
    }
}

impl TokenIssuer for JwtService {
    fn issue_access_token(
        &self,
        user_id: Uuid,
        email: &str,
        status: AccountStatus,
        role: Role,
    ) -> anyhow::Result<String> {
        let now = Utc::now().timestamp();
        let claims = JwtClaims {
            sub: user_id.to_string(),
            email: email.to_owned(),
            status: status.as_str().to_owned(),
            role: Some(role.as_str().to_owned()),
            iat: now,
            exp: now + self.access_ttl,
        };
        encode(&Header::new(Algorithm::RS256), &claims, &self.encoding_key)
            .map_err(|e| anyhow::anyhow!("gagal sign token: {e}"))
    }
}

impl TokenValidator for JwtService {
    fn validate_token(&self, token: &str) -> Result<AuthClaims, AuthClientError> {
        let mut validation = Validation::new(Algorithm::RS256);
        validation.validate_exp = true;

        let data = decode::<JwtClaims>(token, &self.decoding_key, &validation)
            .map_err(|_| AuthClientError::InvalidToken)?;

        let user_id =
            Uuid::parse_str(&data.claims.sub).map_err(|_| AuthClientError::InvalidToken)?;

        Ok(AuthClaims {
            user_id,
            email: data.claims.email,
            status: data.claims.status.parse().ok(),
            role: data.claims.role.as_deref().and_then(|r| r.parse().ok()),
        })
    }
}
