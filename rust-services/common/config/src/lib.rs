//! # common-config — Konfigurasi terpusat untuk semua service rejki-backend
//!
//! Menggantikan scattered `std::env::var()` dengan satu `AppConfig` struct
//! yang dibangun via `config-rs` + `serde`. Mendukung dua pola env var:
//!
//! 1. **Bare env vars** (backward compat): `DATABASE_URL`, `REDIS_URL`, dll.
//! 2. **Prefixed env vars** (baru): `REJKI_DATABASE_URL`, `REJKI_REDIS_URL`, dll.
//!
//! Nilai prefixed (`REJKI_*`) menimpa bare vars, sehingga migration
//! bisa bertahap tanpa break change.

mod clamav;
mod cors;
mod database;
mod jwt;
mod minio;
mod otel;
mod redis;
mod smtp;

pub use clamav::ClamavConfig;
pub use cors::CorsConfig;
pub use database::DatabaseConfig;
pub use jwt::JwtConfig;
pub use minio::MinioConfig;
pub use otel::OtelConfig;
pub use redis::RedisConfig;
pub use smtp::SmtpConfig;

use std::path::PathBuf;

// ── AppEnv ───────────────────────────────────────────────────────────────────

/// Environment aplikasi: development atau production.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppEnv {
    Development,
    Production,
}

impl AppEnv {
    pub fn is_production(&self) -> bool {
        matches!(self, AppEnv::Production)
    }

    pub fn is_development(&self) -> bool {
        matches!(self, AppEnv::Development)
    }
}

// ── AppConfig ────────────────────────────────────────────────────────────────

/// Konfigurasi utama aplikasi — dibaca dari env vars via `config-rs`.
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub app_env: AppEnv,
    pub app_port: u16,

    // ── Sub-konfigurasi per domain ─────────────────────────────────────────
    pub database: DatabaseConfig,
    pub jwt: JwtConfig,
    pub redis: RedisConfig,
    pub minio: MinioConfig,
    pub smtp: SmtpConfig,
    pub otel: OtelConfig,
    pub clamav: ClamavConfig,
    pub cors: CorsConfig,
}

impl AppConfig {
    /// Muat konfigurasi dari environment variables.
    ///
    /// 1. Load `.env` file (development only, silent jika tidak ada).
    /// 2. Baca semua env vars via `config-rs`:
    ///    - Bare env vars (case-insensitive): `DATABASE_URL` → key `database_url`
    ///    - Prefixed (overrides): `REJKI_DATABASE_URL` → key `database_url`
    /// 3. Panic jika field **required** tidak diset (database, minio, dll.).
    ///    Field opsional menggunakan default atau `None`.
    pub fn load() -> Self {
        dotenvy::dotenv().ok();

        let s = config::Config::builder()
            // Backward compat: bare env vars (case-insensitive by default)
            .add_source(config::Environment::default().try_parsing(true))
            // Prefixed vars menimpa bare vars (migrasi bertahap)
            .add_source(
                config::Environment::with_prefix("REJKI")
                    .separator("_")
                    .try_parsing(true),
            )
            .build()
            .expect("gagal build konfigurasi dari environment");

        // ── AppEnv ─────────────────────────────────────────────────────────
        let app_env_raw: String = s.get("app_env").unwrap_or_else(|_| "development".into());
        let app_env = match app_env_raw.to_lowercase().as_str() {
            "production" => AppEnv::Production,
            _ => AppEnv::Development,
        };

        // ── CORS (comma-separated string → Vec) ────────────────────────────
        let cors_raw_str: String = s
            .get("cors_allowed_origins")
            .unwrap_or_else(|_| "http://localhost:5173".into());
        let cors_allowed_origins: Vec<String> = cors_raw_str
            .split(',')
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
            .collect();

        // ── Validate CORS tidak kosong ─────────────────────────────────────
        assert!(
            !cors_allowed_origins.is_empty(),
            "CORS_ALLOWED_ORIGINS tidak boleh kosong — isi dengan daftar origin (comma-separated)"
        );

        Self {
            app_env,
            app_port: s.get("app_port").unwrap_or(8080u16),

            database: DatabaseConfig {
                url: s.get("database_url").expect("DATABASE_URL tidak di-set"),
                pool_size: s.get("database_pool_size").unwrap_or(10u32),
            },

            jwt: JwtConfig {
                private_key_path: s
                    .get("jwt_private_key_path")
                    .unwrap_or_else(|_| PathBuf::from("./keys/private.pem")),
                public_key_path: s
                    .get("jwt_public_key_path")
                    .unwrap_or_else(|_| PathBuf::from("./keys/public.pem")),
                access_ttl_secs: s.get("jwt_access_ttl_secs").unwrap_or(900i64),
                refresh_ttl_secs: s.get("jwt_refresh_ttl_secs").unwrap_or(2_592_000i64),
            },

            redis: RedisConfig {
                url: s.get("redis_url").ok(),
            },

            minio: MinioConfig {
                endpoint: s
                    .get("minio_endpoint")
                    .expect("MINIO_ENDPOINT tidak di-set — MinIO/S3 wajib untuk upload dokumen"),
                access_key: s
                    .get("minio_access_key")
                    .expect("MINIO_ACCESS_KEY tidak di-set"),
                secret_key: s
                    .get("minio_secret_key")
                    .expect("MINIO_SECRET_KEY tidak di-set"),
                bucket: s
                    .get("minio_bucket")
                    .unwrap_or_else(|_| "rejki-dokumen".into()),
                backup_bucket: s.get("minio_backup_bucket").ok(),
            },

            smtp: SmtpConfig {
                host: s.get("smtp_host").ok(),
                port: s.get("smtp_port").unwrap_or(587u16),
                user: s.get("smtp_user").ok(),
                pass: s.get("smtp_pass").ok(),
                email_from: s.get("smtp_email_from").ok(),
            },

            otel: OtelConfig {
                exporter_otlp_endpoint: s.get("otel_exporter_otlp_endpoint").ok(),
                exporter_otlp_headers: s.get("otel_exporter_otlp_headers").ok(),
                service_name: s
                    .get("otel_service_name")
                    .unwrap_or_else(|_| "rejki-backend".into()),
            },

            clamav: ClamavConfig {
                host: s
                    .get("clamav_host")
                    .unwrap_or_else(|_| "clamav-daemon".into()),
                port: s.get("clamav_port").unwrap_or(3310u16),
            },

            cors: CorsConfig {
                allowed_origins: cors_allowed_origins,
            },
        }
    }

    /// Convenience: apakah environment ini production?
    pub fn is_production(&self) -> bool {
        self.app_env.is_production()
    }

    /// Convenience: apakah environment ini development?
    pub fn is_development(&self) -> bool {
        self.app_env.is_development()
    }
}

// ── Load APP_ENV (fail-fast) ─────────────────────────────────────────────────

/// Load `APP_ENV` sekali di awal startup — panic jika tidak valid.
///
/// Dipanggil oleh standalone service yang belum migrate ke `AppConfig::load()`.
pub fn load_app_env() -> AppEnv {
    let raw = std::env::var("APP_ENV").expect("APP_ENV tidak di-set (development | production)");
    match raw.to_lowercase().as_str() {
        "development" => AppEnv::Development,
        "production" => AppEnv::Production,
        other => panic!("APP_ENV tidak valid: '{other}' (harus 'development' atau 'production')"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_app_env_development() {
        std::env::remove_var("APP_ENV");
        std::env::set_var("APP_ENV", "development");
        assert_eq!(load_app_env(), AppEnv::Development);
        std::env::remove_var("APP_ENV");
    }

    #[test]
    fn test_app_env_is_development() {
        let env = AppEnv::Development;
        assert!(env.is_development());
        assert!(!env.is_production());
    }

    #[test]
    fn test_app_env_is_production() {
        let env = AppEnv::Production;
        assert!(env.is_production());
        assert!(!env.is_development());
    }

    #[test]
    fn test_load_app_env_invalid() {
        std::env::set_var("APP_ENV", "staging");
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(load_app_env));
        std::env::remove_var("APP_ENV");
        assert!(result.is_err());
    }
}
