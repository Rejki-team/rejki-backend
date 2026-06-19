## Why

CORS saat ini `allow_origin(Any)` untuk semua environment — development maupun production. Security baseline mensyaratkan whitelist origin eksplisit di production untuk mencegah cross-origin abuse dan memungkinkan `allow_credentials(true)` untuk refresh token HttpOnly cookie di masa depan.

## What Changes

- Env var baru: `CORS_ALLOWED_ORIGINS` — comma-separated list origin yang diizinkan (default: `https://rejki.id,https://app.rejki.id`)
- `common-config/AppConfig` — tambah field `cors_allowed_origins: Vec<String>`
- `main.rs` — CorsLayer conditional: development → `Any`, production → whitelist
- `.env.example` — tambah `CORS_ALLOWED_ORIGINS` dengan placeholder

## Capabilities

### New Capabilities
- `cors-policy`: Environment-aware CORS configuration dengan whitelist untuk production

### Modified Capabilities
- (none — kode baru, tidak mengubah requirements existing)

## Impact

- `common/config/src/lib.rs` — tambah field + parsing `CORS_ALLOWED_ORIGINS`
- `rejki-app/src/main.rs` — conditional CorsLayer berdasarkan `app_env`
- `.env.example` — tambah var baru
- `docs/config-standard.html` — tambah ke katalog
- `docs/security-baseline.html` — konfirmasi implementasi
