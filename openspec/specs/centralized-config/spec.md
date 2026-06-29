## ADDED Requirements

### Requirement: Centralized configuration via AppConfig::load()

Sistem SHALL menggunakan `common_config::AppConfig::load()` sebagai single source of truth untuk semua konfigurasi. Semua service SHALL membaca konfigurasi dari method ini — tidak ada lagi scattered `std::env::var()` di masing-masing service main.

#### Scenario: Load config with DATABASE_URL
- **GIVEN** env var `DATABASE_URL=postgresql://user:pass@localhost/rejki_dev`
- **WHEN** `AppConfig::load()` dipanggil
- **THEN** `cfg.database.url` berisi `postgresql://user:pass@localhost/rejki_dev`
- **AND** `cfg.database.pool_size` berisi default `10`

#### Scenario: Load config missing required field panic
- **GIVEN** `DATABASE_URL` tidak di-set
- **WHEN** `AppConfig::load()` dipanggil
- **THEN** service panic dengan pesan `DATABASE_URL tidak di-set`

### Requirement: APP_ENV auto-detection

Sistem SHALL membaca `APP_ENV` dari environment variable dan mengkategorikannya sebagai `Development` atau `Production`. Default (jika tidak di-set): `Development`.

#### Scenario: Production environment detected
- **GIVEN** `APP_ENV=production`
- **WHEN** `AppConfig::load()` dipanggil
- **THEN** `cfg.app_env` = `AppEnv::Production`
- **AND** `cfg.is_production()` returns `true`

#### Scenario: Default to development
- **GIVEN** `APP_ENV` tidak di-set atau bernilai selain `production`
- **WHEN** `AppConfig::load()` dipanggil
- **THEN** `cfg.app_env` = `AppEnv::Development`

### Requirement: 9 sub-config domains

Sistem SHALL menyediakan sub-struct untuk setiap domain konfigurasi:

| Struct | Env Prefix | Required | Default |
|--------|-----------|----------|---------|
| `DatabaseConfig` | `DATABASE_` | `url` | `pool_size=10` |
| `JwtConfig` | `JWT_` | none | `access_ttl=900`, `refresh_ttl=2_592_000`, keys=`./keys/*.pem` |
| `RedisConfig` | `REDIS_` | none | `url=None` |
| `MinioConfig` | `MINIO_` | `endpoint`, `access_key`, `secret_key` | `bucket=rejki-dokumen` |
| `SmtpConfig` | `SMTP_` | none | `port=587` |
| `OtelConfig` | `OTEL_` | none | `service_name=rejki-backend` |
| `ClamavConfig` | `CLAMAV_` | none | `host=clamav-daemon`, `port=3310` |
| `CorsConfig` | `CORS_` | `allowed_origins` | `http://localhost:5173` |

#### Scenario: All sub-configs accessible from AppConfig
- **WHEN** `AppConfig::load()` berhasil
- **THEN** semua field accessible: `cfg.database`, `cfg.jwt`, `cfg.redis`, `cfg.minio`, `cfg.smtp`, `cfg.otel`, `cfg.clamav`, `cfg.cors`

### Requirement: Prefixed env vars override bare vars

Sistem SHALL mendukung `REJKI_*` prefix yang menimpa bare env vars untuk memungkinkan migration bertahap tanpa break change.

#### Scenario: Prefixed var overrides bare var
- **GIVEN** `DATABASE_URL=postgresql://old` dan `REJKI_DATABASE_URL=postgresql://new`
- **WHEN** `AppConfig::load()` dipanggil
- **THEN** `cfg.database.url` = `postgresql://new` (prefixed menang)

#### Scenario: Bare var used when no prefixed
- **GIVEN** `DATABASE_URL=postgresql://bare` dan tidak ada `REJKI_DATABASE_URL`
- **WHEN** `AppConfig::load()` dipanggil
- **THEN** `cfg.database.url` = `postgresql://bare`

### Requirement: Fail-open for optional services

Konfigurasi untuk service opsional (Redis, SMTP, OTEL, ClamAV) SHALL menggunakan `Option<T>` atau defaults — service tidak panic jika mereka tidak dikonfigurasi.

#### Scenario: Redis not configured — rate limiter disables gracefully
- **GIVEN** `REDIS_URL` tidak di-set
- **WHEN** `AppConfig::load()` dipanggil
- **THEN** `cfg.redis.url` = `None`
- **AND** rate limiter menggunakan `NoOpRateLimiter` (always allow)

#### Scenario: SMTP not configured — email silently skipped
- **GIVEN** `SMTP_HOST` tidak di-set
- **WHEN** `AppConfig::load()` dipanggil
- **THEN** `cfg.smtp.host` = `None`
- **AND** auth service log warning "notifier tidak dikonfigurasi" + tidak mengirim email

### Requirement: Concrete field types instead of raw strings

Sistem SHALL menyimpan konfigurasi sebagai tipe konkret (bukan string mentah): `u16` untuk port, `PathBuf` untuk path file, `Vec<String>` untuk daftar, `i64` untuk TTL.

#### Scenario: CORS origins parsed into Vec
- **GIVEN** `CORS_ALLOWED_ORIGINS=https://rejki.id,https://app.rejki.id`
- **WHEN** `AppConfig::load()` dipanggil
- **THEN** `cfg.cors.allowed_origins` = `["https://rejki.id", "https://app.rejki.id"]`

#### Scenario: JWT TTLs are i64 (seconds)
- **WHEN** `AppConfig::load()` dipanggil
- **THEN** `cfg.jwt.access_ttl_secs` bertipe `i64`
- **AND** `cfg.jwt.refresh_ttl_secs` bertipe `i64`
