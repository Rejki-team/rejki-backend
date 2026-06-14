# PRD — Ekosistem Rejki (Dokumen Payung)

| | |
|---|---|
| **Produk** | Rejki — Platform Marketplace Multi-Vertikal |
| **Dokumen** | Product Requirements Document (Payung) |
**Versi** | 0.2 — Draft |
| **Tanggal** | 2026-06-14 |
| **Status** | Draft untuk ditinjau |
| **Pemilik** | _(belum ditentukan)_ |

> **Catatan akurasi.** Dokumen ini disusun dari dua sumber yang ada di repositori: (1) dokumentasi teknis di `docs/*.html` dan (2) kode sumber di `rust-services/`. Hal yang **tidak tertulis di sumber** (persona, metrik bisnis, KPI, fitur admin & eksekutif) ditandai secara eksplisit sebagai **_(inferensi)_**, **_(USULAN)_**, atau **_(TBD)_**. Tidak ada angka atau target yang dikarang. Asumsi implementasi didokumentasikan di [brainstorm/user-service-phase2.html § Asumsi](../brainstorm/user-service-phase2.html#asumsi).

> **Pembaruan 2026-06-11:** Proposal `extend-auth-service-onboarding` (OpenSpec) selesai diimplementasi dan terverifikasi (build hijau online+offline, 4 unit test + 10 integration test pass, `.sqlx` cache di-generate). Detail asumsi implementasi (parameter OTP, backend kripto, env key, binary standalone) dicatat di dokumen brainstorming dan perlu ditinjau sebelum naik ke produksi.

> **Pembaruan 2026-06-14:** Pemilik produk memberikan **User Story lengkap & otoritatif** untuk **Rejki Web Dashboard** (admin). [prd-dashboard.md](prd-dashboard.md) dinaikkan ke **v0.2** (spesifikasi nyata per menu/halaman, bukan lagi placeholder), dan kebutuhan backend baru dipetakan menjadi proposal OpenSpec terpisah: `add-admin-rbac` (fondasi RBAC + login admin email/password), `extend-iklan-moderation` (moderasi 4 vertikal: status, suspend per-iklan, search/sort, CSV, foto), `add-pelatihan-enrollment-badge` (7 status pelatihan, konfirmasi peserta + bukti transfer, badge/sertifikat), `add-content-reports` (Pengelolaan Dukungan/aduan), `add-corporate-comms` (artikel + broadcast), dan `add-rejki-web-dashboard` (spesifikasi SPA Vue). Keputusan kunci: **satu role `admin` extensible** ke tier, **login admin email+password tanpa OTP** (akun di-inject DBA).

---

## Daftar Dokumen PRD

| Dokumen | Cakupan | Status implementasi |
|---|---|---|
| **rejki-prd.md** _(dokumen ini)_ | Payung: visi, ekosistem, persona, NFR bersama, arsitektur, roadmap | — |
| [prd-mobile.md](prd-mobile.md) | Rejki Mobile — aplikasi pengguna umum | Sebagian besar **sudah ada** di backend |
| [prd-dashboard.md](prd-dashboard.md) | Rejki Web Dashboard — admin/moderasi | **Spesifikasi tersedia (v0.2)** — backend 📋 Rencana per change OpenSpec |
| [prd-ceo.md](prd-ceo.md) | Rejki CEO Mobile — statistik eksekutif | **Rencana** _(USULAN)_ |

---

## 1. Ringkasan Eksekutif

**Rejki** adalah platform *marketplace* multi-vertikal berbahasa Indonesia yang menyatukan empat jenis iklan dalam satu ekosistem: lowongan pekerjaan, profil pekerja/jasa, jual-beli barang bekas, dan pelatihan. Platform ini dilengkapi layanan pendukung berupa autentikasi, profil pengguna, percakapan (chat) real-time, dan notifikasi push.

Ekosistem Rejki terdiri dari **satu backend bersama** (`rejki-backend`) yang melayani **tiga aplikasi klien**:

1. **Rejki Mobile** — untuk pengguna umum yang memasang/mencari iklan, berkomunikasi, dan menerima notifikasi.
2. **Rejki Web Dashboard** — untuk admin yang memoderasi konten dan mengelola platform.
3. **Rejki CEO Mobile** — untuk eksekutif yang memantau statistik bisnis sebagai acuan *canvassing*.

Backend dibangun sebagai **modular monolith** Rust yang siap diekstrak menjadi *microservice* tanpa mengubah *business logic*. Per **2026-06-09**, fondasi (Phase 1) dan standardisasi (Phase 1.5) telah selesai 100%, dengan handler fungsional untuk kedelapan domain.

---

## 2. Latar Belakang & Pernyataan Masalah

Aktivitas ekonomi mikro di Indonesia tersebar di banyak kanal terpisah — lowongan kerja, jasa freelance, jual-beli barang bekas, dan pelatihan sering kali berada di aplikasi atau grup yang berbeda. Hal ini menyulitkan pengguna yang berperan ganda (mis. seorang pencari kerja yang sekaligus menjual barang bekas) dan menyulitkan pelaku usaha menjangkau pasar lokal.

**Masalah yang ingin dipecahkan Rejki:**

- Pengguna butuh **satu tempat** untuk empat kebutuhan marketplace yang umum saling beririsan.
- Pelaku usaha/penyedia jasa butuh **jangkauan berbasis lokasi** untuk menemukan pasar terdekat.
- Operator platform butuh **kontrol moderasi** untuk menjaga kualitas konten.
- Manajemen butuh **visibilitas data** untuk mengambil keputusan ekspansi/*canvassing* per wilayah.

> Pernyataan masalah ini sebagian **_(inferensi)_** dari domain produk; tidak ditemukan dokumen visi/bisnis tertulis di repositori.

---

## 3. Tujuan & Sasaran

### 3.1 Goals
- Menyediakan backend tunggal yang konsisten (standar API, keamanan, observability) untuk seluruh ekosistem Rejki.
- Mendukung empat vertikal iklan + chat + notifikasi untuk pengguna mobile.
- Menyiapkan fondasi yang **extraction-ready** agar tiap domain dapat diskalakan secara independen di masa depan.
- Menyediakan kemampuan moderasi (Dashboard) dan pemantauan bisnis (CEO) — _direncanakan_.

### 3.2 Non-Goals (untuk iterasi saat ini)
- Sistem pembayaran/escrow internal — **tidak** termasuk pada cakupan yang terverifikasi saat ini.
- Microservice fisik terpisah — sengaja ditunda (Phase 4, bila diperlukan).
- Fitur sosial lanjutan (feed, follow, rating/review) — **_(TBD)_**, di luar cakupan kode saat ini.

---

## 4. Peta Ekosistem

| Aplikasi | Pengguna | Teknologi klien | Fungsi inti | Status backend |
|---|---|---|---|---|
| **Rejki Mobile** | Pengguna umum (role user biasa) | **Flutter** (Dart) | Pasang/cari iklan (4 vertikal), chat, notifikasi, profil | ✅ Sebagian besar sudah ada |
| **Rejki Web Dashboard** | Admin | **Vue.js + TypeScript + Tailwind CSS** (repo `rejki-web/`) | Verifikasi KYC, moderasi 4 iklan, siklus pelatihan (verifikasi/konfirmasi/badge), aduan, corporate comms, suspend, CSV | 🔧 Backend sebagian — **6 gap** (audit 2026-06-15) ditutup 3 change baru; UI `add-rejki-web-dashboard` belum dibuat. Lihat [dashboard-gap-analysis.md](../dashboard-gap-analysis.md) |
| **Rejki CEO Mobile** | Eksekutif / CEO | **Flutter** (Dart) | Statistik bisnis, sebaran geografis, tren — acuan canvassing | 📋 Rencana (belum ada) |

Ketiganya berbagi satu backend (`rejki-backend`) dan satu basis data PostgreSQL (schema terisolasi per domain).

> **Sumber teknologi klien.** Flutter untuk Rejki Mobile **tersurat** di dokumen standar Phase 1.5 — [api-standard.html](../api-standard.html) (konversi UTC→lokal via `DateTime.toLocal()`; diagram upload `Flutter → Backend → R2/S3`), [websocket-contract.html](../websocket-contract.html) (diagram klien `Flutter ↔ chat-service`), dan [security-baseline.html](../security-baseline.html). Stack CEO Mobile (Flutter) dan Web Dashboard (Vue + TS + Tailwind) dikonfirmasi pemilik produk. Backend bersifat **client-agnostic** (REST/JSON + WebSocket + envelope standar), sehingga pilihan teknologi klien tidak mengubah kontrak API.

---

## 5. Persona & Target Pengguna

> **_(inferensi)_** — Persona berikut **disimpulkan dari domain bisnis** (jenis iklan & layanan di kode). Repositori **tidak memuat** definisi persona tertulis. Daftar ini perlu divalidasi oleh tim produk.

| Persona | Aplikasi | Kebutuhan utama |
|---|---|---|
| Pencari kerja | Mobile | Menemukan & (kelak) melamar lowongan |
| Perekrut / Perusahaan | Mobile | Memasang lowongan, menjaring kandidat |
| Freelancer / Penyedia jasa | Mobile | Menampilkan keahlian & tarif |
| Klien / Pencari jasa | Mobile | Menemukan pekerja berdasarkan keahlian & lokasi |
| Penjual barang bekas | Mobile | Memasang barang, menandai terjual |
| Pembeli | Mobile | Mencari & menghubungi penjual |
| Penyelenggara pelatihan | Mobile | Mempublikasikan program & jadwal |
| Peserta pelatihan | Mobile | Menemukan & mengikuti pelatihan |
| Admin / Moderator | Web Dashboard | Menjaga kualitas konten & menindak pelanggaran |
| Eksekutif / CEO | CEO Mobile | Memantau kesehatan bisnis & peluang ekspansi |

Detail persona per aplikasi diuraikan di masing-masing sub-PRD.

---

## 6. Lingkup Produk

### 6.1 Domain bisnis (4 vertikal iklan)
1. **Iklan Pekerjaan** — lowongan kerja (judul, perusahaan, gaji, tipe kerja).
2. **Iklan Pekerja** — profil/jasa pekerja (nama, keahlian, tarif).
3. **Iklan Barang Bekas** — jual-beli barang second (harga, kondisi, foto, status terjual).
4. **Iklan Pelatihan** — program pelatihan (penyelenggara, harga, jadwal).

### 6.2 Layanan pendukung
- **Auth** — registrasi, login, OTP, refresh-token rotation (JWT RS256).
- **User** — profil pengguna.
- **Chat** — percakapan & pesan real-time (REST + WebSocket).
- **Notification** — notifikasi in-app + push (Redis Streams → Bun consumer → FCM).

Pemetaan fitur ke endpoint nyata ada di **Lampiran A** dan diperinci di [prd-mobile.md](prd-mobile.md).

---

## 7. Requirement Non-Fungsional (Bersama)

Berlaku untuk seluruh ekosistem. Sebagian besar **sudah diimplementasikan** pada Phase 1.5 (lihat tautan dokumen standar).

| Kategori | Requirement | Status | Rujukan |
|---|---|---|---|
| Keamanan — Auth | JWT RS256 stateless, refresh-token rotation, OTP | ✅ Ada | [security-baseline.html](../security-baseline.html) |
| Keamanan — Otorisasi | Kepemilikan dicek di query SQL; IDOR → **404** (bukan 403) | ✅ Ada | [authorization-pattern.html](../authorization-pattern.html) |
| Keamanan — Input | Validasi `ValidatedJson`, sanitasi HTML (strip XSS) | ✅ Ada | [security-baseline.html](../security-baseline.html) |
| API Standard | Envelope `ApiResponse<T>`, prefix `/api/v1/`, `Location` pada 201 | ✅ Ada | [api-standard.html](../api-standard.html) |
| API Versioning | Kebijakan breaking change, header `Deprecation`/`Sunset` | ✅ Ada | [api-versioning.html](../api-versioning.html) |
| Observability | Logging JSON terstruktur, propagasi `request_id`, peringatan slow-query | ✅ Ada | [logging-standard.html](../logging-standard.html) |
| Real-time | Kontrak WebSocket: auth saat handshake, envelope pesan, close codes | ✅ Ada | [websocket-contract.html](../websocket-contract.html) |
| Notifikasi | Redis Streams + FCM, idempotency, retry/DLQ | ✅ Ada | [notification-contract.html](../notification-contract.html) |
| Ketersediaan | `GET /health`, graceful shutdown SIGTERM (drain) | ✅ Ada | [health-check.html](../health-check.html) |
| Konfigurasi | `APP_ENV` fail-fast, `.env` hanya untuk development | ✅ Ada | [config-standard.html](../config-standard.html) |
| Basis data | snake_case, kolom audit `created_at`/`updated_at`, tanpa cross-schema FK | ✅ Ada | [database-convention.html](../database-convention.html) |
| Pengujian | Integration test dengan PostgreSQL nyata | ✅ Ada | [testing-standard.html](../testing-standard.html) |

---

## 8. Arsitektur & Teknologi (Ringkas)

**Pola:** Modular Monolith Rust yang *extraction-ready* — semua domain dikompilasi menjadi **satu binary** (`rejki-app`), tetapi setiap domain terisolasi (schema DB sendiri, akses antar-domain hanya lewat *trait client*).

**Lapisan (ringkas):**

```
Cloudflare Tunnel  →  nginx (reverse proxy, rate limit)
        →  rejki-app (Composition Root: wire semua router & dependency)
              ├─ auth · user · chat · notification
              └─ iklan: pekerjaan · pekerja · barang-bekas · pelatihan
        →  PostgreSQL (schema terisolasi per domain)
        →  Redis Streams  →  bun-notification-service  →  FCM
```

**Stack:** Rust · Axum 0.8 · Tokio · PostgreSQL 17 (sqlx) · Redis 7 · Bun.js (consumer FCM) · Docker Compose · nginx · Cloudflare Tunnel.

Detail lengkap: [index.html](../index.html) dan [infrastructure-setup.html](../infrastructure-setup.html).

---

## 9. Asumsi, Batasan, dan Dependensi

**Asumsi**
- Satu pengguna dapat berperan ganda (mis. penjual sekaligus pencari kerja).
- Identitas pengguna berbasis email + OTP.

**Batasan**
- Backend saat ini **tidak memiliki konsep peran (role/RBAC)** — semua pengguna setara. Ini menjadi prasyarat untuk Dashboard & CEO (lihat §10).
- Belum ada layanan agregasi statistik/analytics.

**Dependensi eksternal**
- **Firebase Cloud Messaging (FCM)** — pengiriman push notification.
- **SMTP** — pengiriman email/OTP.
- **Cloudflare Tunnel** — edge & akses tanpa membuka port publik.

---

## 10. Status Implementasi & Roadmap

| Phase | Isi | Status |
|---|---|---|
| **Phase 1 — Foundation** | Workspace, common crates, auth-service, skeleton 8 domain | ✅ Selesai |
| **Phase 1.5 — Standardization** | 14 standar wajib (W0–W9): API, security, logging, WebSocket, notifikasi, infra, CI/CD, DB, testing, config, health, versioning, authorization | ✅ Selesai (2026-06-09) |
| **Phase 2 — Core Services** | Pendalaman fitur per domain (search/filter, update, media, pagination lanjutan) | 📋 Rencana |
| **Phase 3 — Hardening** | Rate limiting, proteksi brute-force OTP, observability produksi | 📋 Rencana |
| **Phase 4 — Extraction** | Ekstraksi domain ke microservice (bila diperlukan) | 📋 Rencana |

### Kebutuhan backend baru untuk Dashboard & CEO _(Status per 2026-06-15)_

Untuk **Dashboard**, sebagian besar backend telah diimplementasikan per change OpenSpec; **3 change baru (#11–#13)** menutup gap hasil audit 2026-06-15:

| # | Change | Status | Review |
|---|--------|--------|--------|
| 1 | **`add-admin-rbac`** — kolom `role`, login admin, middleware `require_admin`, seed admin | ✅ Selesai | [review](../code-review/add-admin-rbac-review.md) |
| 2 | **`extend-auth-service-onboarding`** — state machine 7-status, password policy, OTP, suspend akun, feature gating | ✅ Selesai | [review](../code-review/extend-auth-service-onboarding-review.md) |
| 3 | **`add-user-service-kyc`** — profil KYC, NIK encrypted, dokumen KTP/swafoto, verifikasi manual, audit trail | ✅ Selesai | [review](../code-review/add-user-service-kyc-review.md) |
| 4 | **`add-region-service`** — data wilayah 4 tingkat Indonesia, cascading API, validasi rantai | ✅ Selesai | [review](../code-review/add-region-service-review.md) |
| 5 | **`add-pelatihan-enrollment-badge`** — 7 status, enrollment + bukti transfer, badge + sertifikat | ✅ Selesai | [review](../code-review/add-pelatihan-enrollment-badge-review.md) |
| 6 | **`add-corporate-comms`** — artikel korporat + broadcast notifikasi via `AuthClient.list_active_user_ids()` | ✅ Selesai | [review](../code-review/add-corporate-comms-review.md) |
| 7 | **`add-content-reports`** — domain report/aduan + report-service + report-service-client | ✅ Selesai | — |
| 8 | **`add-storage-service-spec`** — dokumentasi formal kontrak StorageClient + 8 kategori | ✅ Selesai | [review](../code-review/add-storage-service-spec-review.md) |
| 9 | **`extend-iklan-moderation`** — state moderasi + suspend per-iklan + search/sort + CSV export | ✅ Selesai | — |
| 10 | **`add-rejki-web-dashboard`** — SPA dashboard (UI/UX) | 📋 Rencana (web) | — |
| 11 | **`add-user-admin-management`** — listing KYC admin, baca dokumen teraudit, auto-purge saat reject | 🔴 Baru (gap audit 2026-06-15) | — |
| 12 | **`extend-user-suspension-bulk-purge`** — bulk suspend pengguna + purge dokumen saat permanen | 🔴 Baru (gap audit 2026-06-15) | — |
| 13 | **`extend-barang-bekas-gratis-model`** — model gratis/donasi (jenis/jumlah/lokasi pengambilan/Sudah Diambil) | 🔴 Baru (gap audit 2026-06-15) | — |

> **Audit gap 2026-06-15.** Penelusuran kode ulang menemukan 6 gap backend yang sebelumnya keliru ditandai selesai; ditutup oleh change #11–#13. Detail & bukti baris kode: [dashboard-gap-analysis.md](../dashboard-gap-analysis.md).

Untuk **CEO**, masih dibutuhkan **layer analytics/reporting** read-only (agregasi via view/materialized view atau service statistik terpisah); normalisasi wilayah tersedia via `add-region-service`.

Rincian Dashboard ada di [prd-dashboard.md](prd-dashboard.md) (v0.2, dengan keterlacakan FR → change OpenSpec) dan CEO di [prd-ceo.md](prd-ceo.md).

---

## 11. Pertanyaan Terbuka & Risiko

**Pertanyaan terbuka**
- Apakah Rejki akan memiliki sistem pembayaran/transaksi internal? (memengaruhi metrik "transaksi" untuk CEO)
- Tingkatan peran admin yang diinginkan (Super Admin / Moderator / Support)?
- Apakah CEO Mobile murni read-only atau perlu aksi (mis. set target, ekspor)?
- Definisi "pengguna aktif" dan periode retensi yang dipakai bisnis?

**Risiko**
- Menambah RBAC menyentuh `auth` & seluruh handler → perlu desain middleware peran yang cermat.
- Agregasi statistik di DB transaksional dapat membebani; mungkin perlu replika baca / materialized view.
- Persona & metrik masih asumsi → risiko *misalignment* bila tidak divalidasi tim produk.

---

## Lampiran A — Ringkasan Endpoint API (yang sudah ada)

> Prefix global `/api/v1/`. Diturunkan langsung dari `router()` & handler tiap domain.

**Auth** (`/api/v1/auth`)
- `POST /register`, `POST /login`, `POST /verify-otp`, `POST /resend-otp`, `POST /refresh`, `POST /logout`

**User** (`/api/v1/users`)
- `GET /me`, `PATCH /me`, `GET /{id}`

**Chat** (`/api/v1/chat`)
- `POST /conversations`, `GET /conversations/{id}/messages`, `POST /conversations/{id}/messages`, `GET /ws` (WebSocket)

**Notification** (`/api/v1/notif`)
- `GET /` (list), `POST /send`, `PATCH /{id}/read` (mark read)

**Iklan** — pola identik untuk keempat vertikal:
- `/api/v1/pekerjaan`, `/api/v1/pekerja`, `/api/v1/barang`, `/api/v1/pelatihan`
- `GET /` (list), `POST /` (create → 201 + `Location`), `GET /{id}`, `DELETE /{id}` (cek kepemilikan, IDOR → 404), `GET /health`

**Global**
- `GET /health`

---

## Lampiran B — Ringkasan Entity & Field (dari kode)

| Entity | Field |
|---|---|
| **IklanPekerjaan** | id, poster_id, judul, perusahaan, deskripsi, lokasi?, gaji_min?, gaji_max?, tipe (full_time/part_time/freelance/internship), is_active, created_at, updated_at |
| **IklanPekerja** | id, poster_id, nama, keahlian[], deskripsi, lokasi?, tarif_min?, tarif_max?, is_active, created_at, updated_at |
| **IklanBarangBekas** | id, seller_id, judul, deskripsi, harga, kondisi (baru/sangat_baik/baik/cukup), lokasi?, foto_urls[], is_sold, created_at, updated_at |
| **IklanPelatihan** | id, poster_id, judul, penyelenggara, deskripsi, lokasi?, harga?, tanggal_mulai?, tanggal_selesai?, status (7 nilai), created_by_role, jumlah_peserta, reviewed_by, review_note, deleted_at, is_active, moderation_status, foto_urls, created_at, updated_at |
| **auth.users** | id, email, password_hash, phone (enc), status, role, tos_accepted_at, tos_version, created_at, updated_at |
| **region.*** | province, regency, district, village (4 tabel dengan kode wilayah TEXT PK + parent_id FK) |
| **user_svc.profiles** | id, auth_id, username, full_name, avatar, bio, phone, nik_encrypted, nik_last4, education_level, gender, birth_date, address_line, country_code, province_id, regency_id, district_id, village_id, created_at, updated_at |
| **user_svc.kyc_submission** | id, profile_id, status (pending/approved/rejected), ktp_object_key, selfie_object_key, reviewed_by, review_note, reviewed_at, created_at, updated_at |
| **user_svc.document_access_log** | actor_id, object_key, action (upload_issued/commit/read_issued), request_id, occurred_at |

_`?` menandai field opsional (`Option<T>`); `[]` menandai array._

### Entity baru Dashboard _(✅ Terimplementasi)_

| Entity | Field inti | Change | Status |
|---|---|---|---|
| **auth.users.role** | `role TEXT NOT NULL DEFAULT 'user'` (CHECK `user`/`admin`, extensible) | `add-admin-rbac` | ✅ |
| **auth.account_suspension** | id, user_id, is_permanent, reason, evidence_object_key, expires_at, created_by, created_at | `extend-auth-service-onboarding` | ✅ |
| **iklan.\*.moderation** | tambah `moderation_status` (active/suspended_temp/suspended_permanent), `deleted_at` | `extend-iklan-moderation` | ✅ |
| **iklan_suspension** | id, iklan_id, is_permanent, reason, evidence_object_key, expires_at, created_by, created_at | `extend-iklan-moderation` | ✅ |
| **pelatihan (moderation)** | tambah `status` (7 nilai), `created_by_role`, `jumlah_peserta`, `reviewed_by`, `review_note` | `add-pelatihan-enrollment-badge` | ✅ |
| **pelatihan_enrollment** | id, pelatihan_id, user_id, bukti_transfer_object_key, status, reviewed_by, review_note, timestamps + UNIQUE(pelatihan_id, user_id) | `add-pelatihan-enrollment-badge` | ✅ |
| **pelatihan_badge** | id, pelatihan_id, user_id, sertifikat_object_key, approved_at, status, reviewed_by, review_note, timestamps + UNIQUE(pelatihan_id, user_id) | `add-pelatihan-enrollment-badge` | ✅ |
| **report** | id, reporter_id, target_type (iklan/user), target_id, keterangan, evidence_object_key, status, action_note, reviewed_by, timestamps | `add-content-reports` | ✅ |
| **corporate_article** | id, author_id, category, title, body, photo_object_key, deleted_at, created_at, updated_at | `add-corporate-comms` | ✅ |

---

## Lampiran C — Indeks Dokumen Standar (`docs/`)

[index.html](../index.html) · [phase-1-setup.html](../phase-1-setup.html) · [implementation-plan-phase-1.5.html](../implementation-plan-phase-1.5.html) · [api-standard.html](../api-standard.html) · [security-baseline.html](../security-baseline.html) · [logging-standard.html](../logging-standard.html) · [websocket-contract.html](../websocket-contract.html) · [notification-contract.html](../notification-contract.html) · [bun-notification-service.html](../bun-notification-service.html) · [infrastructure-setup.html](../infrastructure-setup.html) · [vps-deployment.html](../vps-deployment.html) · [cicd-setup.html](../cicd-setup.html) · [database-convention.html](../database-convention.html) · [testing-standard.html](../testing-standard.html) · [config-standard.html](../config-standard.html) · [health-check.html](../health-check.html) · [api-versioning.html](../api-versioning.html) · [authorization-pattern.html](../authorization-pattern.html)
