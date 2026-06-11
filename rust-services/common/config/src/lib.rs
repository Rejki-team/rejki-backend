use std::env;

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

/// Load APP_ENV once at startup — panics if not set or invalid.
/// All other env reads happen via `AppConfig::from_env`.
pub fn load_app_env() -> AppEnv {
    match env::var("APP_ENV")
        .expect("APP_ENV tidak di-set (development | production)")
        .as_str()
    {
        "development" => AppEnv::Development,
        "production" => AppEnv::Production,
        other => panic!("APP_ENV tidak valid: '{other}' (harus 'development' atau 'production')"),
    }
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub app_env: AppEnv,
    pub app_port: u16,
    pub database_url: String,
    pub redis_url: String,
    pub service_name: String,
}

impl AppConfig {
    /// Baca semua env var yang dibutuhkan — panics jika ada yang hilang.
    pub fn from_env(app_env: AppEnv) -> Self {
        if app_env == AppEnv::Development {
            dotenvy::dotenv().ok();
        }

        Self {
            app_port: env::var("APP_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(8080),
            database_url: env::var("DATABASE_URL").expect("DATABASE_URL tidak di-set"),
            redis_url: env::var("REDIS_URL").expect("REDIS_URL tidak di-set"),
            service_name: env::var("SERVICE_NAME").unwrap_or_else(|_| "rejki-app".into()),
            app_env,
        }
    }
}
