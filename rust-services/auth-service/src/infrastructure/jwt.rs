use chrono::Utc;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use auth_service_client::{AuthClaims, AuthClientError};

use crate::domain::entity::AccountStatus;

/// Internal JWT claims structure (superset dari AuthClaims publik).
#[derive(Debug, Serialize, Deserialize)]
struct JwtClaims {
    sub: String, // user_id sebagai string
    email: String,
    status: String, // status akun (untuk gating cepat)
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

    /// Issue access token untuk user (menyertakan status akun untuk gating cepat).
    pub fn issue_access_token(
        &self,
        user_id: Uuid,
        email: &str,
        status: AccountStatus,
    ) -> anyhow::Result<String> {
        let now = Utc::now().timestamp();
        let claims = JwtClaims {
            sub: user_id.to_string(),
            email: email.to_owned(),
            status: status.as_str().to_owned(),
            iat: now,
            exp: now + self.access_ttl,
        };
        encode(&Header::new(Algorithm::RS256), &claims, &self.encoding_key)
            .map_err(|e| anyhow::anyhow!("gagal sign token: {e}"))
    }

    /// Validasi token dan kembalikan claims. Dipakai oleh AuthInProcessClient.
    pub fn validate_token(&self, token: &str) -> Result<AuthClaims, AuthClientError> {
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
        })
    }
}
