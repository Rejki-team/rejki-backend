## Context

`rejki-app` (rust-services/rejki-app/src/main.rs) adalah composition root Modular Monolith berbasis axum 0.8; ia menggabungkan router seluruh service di bawah `/api/v1/*` plus `/health`. Konfigurasi runtime dibaca via `common/config` yang mengekspos `AppEnv { Development, Production }` dengan `is_production()`. Belum ada dependency/route OpenAPI di codebase.

Di sisi operasional, satu VPS menjalankan dua instance Docker Compose terpisah: `/opt/rejki/dev` (project `rejki-dev`, `APP_ENV=development`) dan `/opt/rejki/prod` (project `rejki-prod --profile prod`, `APP_ENV=production`), dengan port host & database berbeda (terdokumentasi di `docs/vps-deployment.html` section 10). Pemilik produk mengonfirmasi: di konteks VPS, dev & prod pada dasarnya setara; pembeda hanya subdomain-vs-domain, Swagger dev-only, dan sedikit beda nginx — sehingga secret/variable CI pada dasarnya sama.

## Goals / Non-Goals

**Goals:**
- Menyajikan Swagger UI + dokumen OpenAPI di `rejki-app` hanya saat `APP_ENV=development`, di-gate saat pembentukan router (route tidak terdaftar di production).
- Menyediakan dokumen OpenAPI dasar (metadata + `/health`) yang bisa bertumbuh.
- Menambah `AppEnv::is_development()` sebagai gate eksplisit.
- Menyelaraskan dokumentasi environment (deployment, infrastructure, cicd, `.env.example`) agar pembeda dev/prod akurat & seragam.

**Non-Goals:**
- Menganotasi seluruh handler tiap service dengan `utoipa::path` (binding penuh). Cukup doc dasar; pertumbuhan per-endpoint dilakukan inkremental di luar change ini.
- Mengubah kontrak atau perilaku endpoint bisnis `/api/v1/*`.
- Menambah autentikasi/akses-kontrol khusus untuk Swagger di dev (cukup dibatasi pada level jaringan: SSH tunnel / Cloudflare Access sesuai praktik deployment).
- Mengimplementasikan mekanisme deploy/CI baru — hanya menyelaraskan dokumentasinya.

## Decisions

- **D1 — utoipa + utoipa-swagger-ui (bukan utoipa-axum penuh).** Pakai `utoipa` 5 (`axum_extras`, `uuid`, `chrono`) untuk `#[derive(OpenApi)]` dan `utoipa-swagger-ui` 9 (fitur `axum`) untuk `SwaggerUi` router. Alternatif `utoipa-axum` (OpenApiRouter mengikat tiap handler) ditolak karena menuntut anotasi menyeluruh ke semua service sekaligus — lingkup terlalu besar dan invasif. Pendekatan terpilih membiarkan doc tumbuh inkremental.
- **D2 — Gate di tingkat router, bukan middleware.** Saat `cfg.app_env.is_development()` true, `rejki-app` me-`merge` `SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi())` ke router; saat production, blok ini dilewati sehingga rute benar-benar absen (404), bukan disembunyikan. Memenuhi spec "tidak dipasang sama sekali".
- **D3 — `AppEnv::is_development()`** ditambahkan melengkapi `is_production()`; gate memakai metode ini agar niat terbaca jelas dan tidak bergantung pada negasi.
- **D4 — Dokumentasi sebagai sumber kebenaran environment.** Pembeda dev/prod ditulis sekali secara konsisten di `vps-deployment.html` section 10, lalu dirujuk/diselaraskan oleh `infrastructure-setup.html`, `cicd-setup.html`, dan `.env.example`. Menghindari deskripsi yang menyimpang antar dokumen.
- **D5 — Versi OpenAPI mengikuti versi crate `rejki-app`.** Metadata versi diambil dari `env!("CARGO_PKG_VERSION")` agar selaras rilis.
- **D6 — Pembatasan akses Swagger dev: berlapis (network-first), bukan hanya gate aplikasi.** (Jawaban Open Question Q1.) Standar industri terkini untuk dokumentasi API non-produksi adalah pertahanan berlapis: (a) **gate by environment** — nonaktif total di production [sudah dipenuhi D2]; (b) **batasan jaringan** — akses dev dibatasi (subdomain dev di belakang Cloudflare Access / IP allowlist / SSH tunnel), tidak publik; (c) **selalu HTTPS** [terpenuhi via Cloudflare]; (d) **`openapi.json` ikut diproteksi**, bukan hanya UI — karena attacker bisa menarik skema penuh langsung dari JSON meski UI diblokir. Keputusan: untuk change ini cukup **gate environment (aplikasi) + pembatasan jaringan di dev (Cloudflare Access pada subdomain dev sebagai default, SSH tunnel sebagai alternatif)**; basic-auth di nginx dev TIDAK dipakai (Cloudflare Access lebih baik: SSO, audit, tanpa shared secret). Auth-level untuk Swagger (mis. OAuth pada UI) dianggap berlebihan karena di production Swagger absen sepenuhnya.
- **D7 — Sertakan satu contoh endpoint auth sebagai teladan anotasi, tanpa membocorkan `utoipa` ke crate domain.** (Jawaban Open Question Q2.) Selain `/health`, dokumentasikan `POST /api/v1/auth/login` sebagai teladan pola anotasi. Kendala: `LoginInput`/`TokenPair` berada di crate `auth-service`; menambah `#[derive(ToSchema)]` di sana memaksa `auth-service` menarik `utoipa` (membocorkan dependency dokumentasi ke domain — melanggar semangat D1/Non-Goals). Keputusan: definisikan **schema cermin (mirror) lokal** di `rejki-app` khusus dokumentasi (mis. `LoginDocRequest`/`LoginDocResponse` ber-`ToSchema`) yang mereplikasi bentuk payload, dipakai hanya di `#[utoipa::path]`. Dengan begitu crate domain tetap bersih dan `rejki-app` (composition root) menanggung beban dokumentasi. Trade-off: ada duplikasi bentuk payload yang harus dijaga sinkron — diterima karena lingkup kecil & terlokalisasi; bila cakupan OpenAPI tumbuh besar, pertimbangkan kembali memindahkan `ToSchema` ke crate domain di belakang feature-flag.

## Risks / Trade-offs

- **Build lebih berat (swagger-ui assets di-bundle).** → Aset hanya menambah ukuran binary; rute tidak aktif di prod sehingga tak ada biaya runtime di production. Dapat ditinjau ulang bila ukuran image jadi masalah.
- **Doc OpenAPI dasar bisa "bohong" bila tidak dirawat** (mendokumentasikan sedikit endpoint, terlihat tidak lengkap). → Tegaskan di doc & spec bahwa cakupan tumbuh inkremental; mulai dari `/health` + metadata jujur.
- **Risiko Swagger bocor ke prod akibat salah set `APP_ENV`.** → `APP_ENV` wajib & fail-fast (sudah ada di `load_app_env`); gate memakai `is_development()` eksplisit; ditambah catatan operasional bahwa dev hanya diakses via subdomain terbatas.
- **Dokumentasi HTML diedit manual** (bukan generated). → Edit terfokus pada section terkait; jaga konsistensi istilah lintas dokumen (D4).
- **`openapi.json` bocor walau UI diblokir.** → Karena gate D2 men-`merge` seluruh `SwaggerUi` (UI + URL `openapi.json`) dalam satu blok `is_development()`, dokumen JSON pun absen di production. Pembatasan jaringan dev (D6) berlaku ke kedua rute, bukan UI saja.
- **Schema mirror auth bisa menyimpang dari DTO asli** (D7). → Jaga sinkron via review; mirror sengaja minimal (hanya field yang diekspos). Bila menyimpang, dampaknya hanya akurasi dokumentasi dev, bukan perilaku runtime.

## Migration Plan

1. Tambah dependency `utoipa` & `utoipa-swagger-ui` di workspace + `rejki-app`.
2. Tambah `AppEnv::is_development()` di `common/config`.
3. Definisikan `ApiDoc` (derive `OpenApi`) + mount `SwaggerUi` di `rejki-app` di belakang gate `is_development()`.
4. Verifikasi: `APP_ENV=development` → Swagger UI & `openapi.json` 200; `APP_ENV=production` → 404; `/health` & `/api/v1/*` tak berubah; `cargo build`/`clippy -D warnings` hijau.
5. Selaraskan dokumentasi: `vps-deployment.html` (section 10), `.env.example`, `infrastructure-setup.html`, `cicd-setup.html`.
6. Rollback: hapus blok gate Swagger + dependency; tidak ada perubahan data/skema, jadi rollback aman & tanpa migrasi.

## Open Questions

_Tidak ada yang terbuka — kedua pertanyaan awal telah diputuskan:_

- **Q1 — Pembatasan akses Swagger dev** → diputuskan di **D6**: pertahanan berlapis (gate environment + pembatasan jaringan dev via Cloudflare Access pada subdomain, SSH tunnel sebagai alternatif; `openapi.json` ikut tergate; selalu HTTPS). Mengikuti standar industri terkini untuk dokumentasi API non-produksi.
- **Q2 — Contoh endpoint auth di OpenAPI awal** → diputuskan di **D7**: ya, sertakan `POST /api/v1/auth/login` sebagai teladan anotasi, memakai schema mirror lokal di `rejki-app` agar crate domain tidak menarik `utoipa`.
