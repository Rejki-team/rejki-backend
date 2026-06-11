## 1. Dependency & Konfigurasi

- [x] 1.1 Tambah `utoipa` 5 (features: `axum_extras`, `uuid`, `chrono`) & `utoipa-swagger-ui` 9 (feature: `axum`) ke `[workspace.dependencies]` di `rust-services/Cargo.toml`
- [x] 1.2 Tarik `utoipa` & `utoipa-swagger-ui` sebagai dependency di `rust-services/rejki-app/Cargo.toml`
- [x] 1.3 Tambah `AppEnv::is_development()` di `rust-services/common/config/src/lib.rs` (melengkapi `is_production()`)

## 2. Implementasi Swagger (dev-only) di rejki-app

- [x] 2.1 Definisikan struct `ApiDoc` dengan `#[derive(utoipa::OpenApi)]`: metadata judul/deskripsi, versi dari `env!("CARGO_PKG_VERSION")`, dan path health-check
- [x] 2.2 Anotasi/registrasi endpoint `/health` ke dokumen OpenAPI (mis. `#[utoipa::path]` pada handler health atau referensi path setara)
- [x] 2.3 (D7) Tambah teladan anotasi `POST /api/v1/auth/login`: buat schema mirror lokal di `rejki-app` (mis. `LoginDocRequest { email, password }` & `LoginDocResponse { access_token, refresh_token, token_type, expires_in }`) ber-`ToSchema`, lalu `#[utoipa::path]` yang merujuknya; registrasikan path & schema ke `ApiDoc`. Crate `auth-service` TIDAK menarik `utoipa`
- [x] 2.4 Di `main.rs`, bila `cfg.app_env.is_development()` true, merge `SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi())` ke router; bila production, blok dilewati (rute tidak terdaftar)
- [x] 2.5 Pastikan gate tidak menyentuh komposisi `/api/v1/*` & `/health` (urutan layer/merge aman)

## 3. Verifikasi

- [x] 3.1 `APP_ENV=development`: `GET /swagger-ui` → halaman UI; `GET /api-docs/openapi.json` → dokumen OpenAPI valid (memuat judul, versi, path health, dan path `auth/login`)
- [x] 3.2 `APP_ENV=production`: `GET /swagger-ui` DAN `GET /api-docs/openapi.json` → 404 (kedua rute absen — skema JSON tidak boleh bocor walau UI saja diblokir, D6)
- [x] 3.3 `GET /health` & rute `/api/v1/*` berperilaku sama di kedua environment
- [x] 3.4 `cargo build` rejki-app hijau & `cargo clippy -p rejki-app -- -D warnings` bersih (kode change ini); 13/13 integration test tetap lulus. Catatan: clippy `--all-targets -D warnings` workspace-wide menabrak error PRE-EXISTING di `common-crypto` (deprecated `aes-gcm from_slice`) & `region-service-client` yang tidak terkait change ini (file untracked, di luar scope)

## 4. Penyelarasan Dokumentasi Environment

- [x] 4.1 `docs/vps-deployment.html` section 10: tambahkan pembeda subdomain (dev) vs domain (prod), Swagger dev-only, dan catatan secret/variable GitHub Actions sama untuk dev & prod (beda nilai saja)
- [x] 4.2 `.env.example`: perjelas struktur variabel dev & prod di VPS sama; pembeda hanya domain/subdomain, `APP_ENV`, dan ketersediaan Swagger; tambahkan/penjelas var domain bila relevan
- [x] 4.3 `docs/infrastructure-setup.html`: selaraskan deskripsi nginx dev vs prod dan sebutkan toggle Swagger dev-only (konsisten dengan section 10)
- [x] 4.4 `docs/cicd-setup.html`: catat bahwa secret & variable GitHub Actions seragam untuk dev & prod; hanya nilai environment-specific (domain/subdomain, APP_ENV, token) yang berbeda
- [x] 4.5 (D6) Dokumentasikan pembatasan akses Swagger dev: subdomain dev di belakang Cloudflare Access (default) atau SSH tunnel (alternatif), selalu via HTTPS; tegaskan `openapi.json` ikut absen di prod. Tulis di `vps-deployment.html` (section 10) dan rujuk di `infrastructure-setup.html`
- [x] 4.6 Periksa konsistensi istilah lintas dokumen (dev=subdomain+Swagger+akses terbatas, prod=domain tanpa Swagger) agar tidak ada deskripsi yang menyimpang

## 5. Finalisasi

- [x] 5.1 Jalankan `openspec validate add-swagger-dev-and-env-docs` hingga valid
- [x] 5.2 Pastikan tidak ada referensi usang yang menyebut "Swagger belum ada" atau pembeda environment yang lama di dokumentasi yang tersentuh (diverifikasi via grep; `index.html` hanya tabel perbandingan framework, bukan status proyek)
