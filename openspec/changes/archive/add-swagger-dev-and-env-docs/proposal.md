## Why

Backend belum punya dokumentasi API interaktif, sehingga pengembang frontend/mobile dan QA harus membaca kode untuk mengetahui kontrak endpoint. Selain itu, dokumentasi environment proyek belum konsisten menjelaskan model "dua environment (dev & prod) berdampingan di satu VPS": pembeda nyata (subdomain vs domain, ketersediaan Swagger, secret/variable CI yang pada dasarnya sama) belum tertulis rapi. Keduanya menghambat onboarding dan deploy yang benar.

## What Changes

- **Swagger UI dev-only**: `rejki-app` menyajikan Swagger UI + dokumen OpenAPI **hanya** saat `APP_ENV=development`. Saat `APP_ENV=production`, route Swagger **tidak dipasang sama sekali** (gated saat wiring router, bukan sekadar disembunyikan).
- **Dokumen OpenAPI dasar**: definisi `OpenApi` (judul, versi, deskripsi, server) + beberapa path inti (mis. `/health`) menggunakan `utoipa`. Doc ini dirancang untuk bertumbuh; tidak mengikat seluruh handler service sekarang.
- **`AppEnv::is_development()`**: ditambahkan di `common/config` (melengkapi `is_production()` yang sudah ada) sebagai gate eksplisit.
- **Penyelarasan dokumentasi environment** agar akurat & terstruktur:
  - `docs/vps-deployment.html` (section 10): tambahkan pembeda **subdomain (dev) vs domain (prod)**, **Swagger dev-only**, dan catatan bahwa **secret/variable GitHub Actions pada dasarnya sama** untuk dev & prod (beda nilai saja).
  - `.env.example`: perjelas bahwa struktur variabel dev & prod di VPS sama; pembeda hanya domain/subdomain, `APP_ENV`, dan ketersediaan Swagger.
  - `docs/infrastructure-setup.html`: selaraskan deskripsi nginx dev vs prod dan toggle Swagger.
  - `docs/cicd-setup.html`: catat secret & variable GitHub Actions sama untuk dev & prod.

Tidak ada perubahan kontrak endpoint bisnis; ini menambah dokumentasi (interaktif + tertulis), bukan mengubah perilaku API.

## Capabilities

### New Capabilities
- `api-docs-swagger`: penyediaan dokumentasi API interaktif (OpenAPI + Swagger UI) yang aktif hanya pada environment development dan absen pada production.
- `environment-topology`: definisi resmi topologi environment proyek (local dev, dev-di-VPS, prod-di-VPS) beserta pembeda yang mengikat: subdomain vs domain, Swagger dev-only, dan kesamaan secret/variable CI antar environment.

### Modified Capabilities
<!-- Tidak ada: belum ada main specs yang requirement-nya berubah; perubahan dokumentasi diatur oleh kapabilitas environment-topology yang baru. -->

## Impact

- **Kode**:
  - `rust-services/Cargo.toml` (workspace deps): tambah `utoipa` 5 (`axum_extras`, `uuid`, `chrono`) & `utoipa-swagger-ui` 9 (`axum`).
  - `rust-services/rejki-app/Cargo.toml`: tarik kedua dependency.
  - `rust-services/rejki-app/src/main.rs`: definisi `OpenApi` + mount `SwaggerUi` di belakang gate `cfg.app_env.is_development()`.
  - `rust-services/common/config/src/lib.rs`: tambah `AppEnv::is_development()`.
- **API / Routing**: rute baru hanya-dev `GET /swagger-ui` (+ `…/openapi.json`); tidak ada di production. Tidak menyentuh endpoint `/api/v1/*` yang ada.
- **Dokumentasi**: `docs/vps-deployment.html`, `docs/infrastructure-setup.html`, `docs/cicd-setup.html`, `.env.example`.
- **Build/Deploy**: dependency build bertambah (utoipa + swagger-ui assets). Tidak mengubah image produksi secara fungsional (route Swagger tidak aktif saat `APP_ENV=production`).
- **Keamanan**: permukaan API production tidak bertambah — Swagger tidak terekspos di prod. Subdomain dev sebaiknya dibatasi akses (mis. via SSH tunnel / Cloudflare Access), konsisten dengan praktik di doc deployment.
