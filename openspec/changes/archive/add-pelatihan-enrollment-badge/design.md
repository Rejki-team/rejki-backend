## Context

`iklan-pelatihan-service` saat ini adalah service CRUD sederhana: entity `IklanPelatihan` (judul, penyelenggara, deskripsi, lokasi?, harga?, tanggal_mulai?, tanggal_selesai?, is_active) dengan route create/list/get/delete. Field `lokasi`/`harga`/`tanggal_*` ada di DTO tetapi **tidak ikut di-INSERT** (bug). Tidak ada konsep status moderasi, peserta/enrollment, badge, atau sertifikat. `StorageClient` mendukung presigned upload; `NotificationClient` mendukung `send`/`send_bulk`/`send_email`. `add-admin-rbac` menyediakan `require_admin`. `extend-iklan-moderation` tidak mengelola lifecycle pelatihan (koordinasi: kolom moderasi dasar di extend-iklan, lifecycle 7-status di proposal ini). Acuan: FR-ADM-TRN-*, FR-ADM-CNF-*, FR-ADM-BDG-* di PRD Dashboard + FR-MOB-TRN-07/08 (prasyarat mobile).

## Goals / Non-Goals

**Goals:** 7 status pelatihan + auto-approve admin vs review user; enrollment + bukti transfer + review; badge + sertifikat + review; listing admin ketiga sub-halaman (search/sort/CSV); perbaikan insert field.

**Non-Goals:** Pembayaran/escrow, penerbitan sertifikat otomatis, penyelarasan `lokasi` ke region, moderasi lintas-vertikal, UI dashboard.

## Decisions

### D1 — Status pelatihan = 7 nilai state machine terpisah dari `moderation_status`
Menambah status fungsional pelatihan (7 nilai) yang menggantikan/melengkapi `is_active` binary. Status mengikuti lifecycle diagram [pelatihan-lifecycle.puml](../../../docs/diagrams/pelatihan-lifecycle.puml):
1. `verifikasi_tertunda` — pengajuan baru oleh user
2. `verifikasi_dalam_proses` — admin mulai tinjau
3. `verifikasi_ditolak` — terminal
4. `verifikasi_diterima` — admin setujui (atau auto-approve admin)
5. `pelatihan_belum_dimulai` — sebelum tanggal_mulai
6. `pelatihan_berjalan` — setelah tanggal_mulai
7. `pelatihan_selesai` — setelah tanggal_selesai

Transisi 5→6 dan 6→7 dapat diotomatisasi berdasarkan tanggal (atau manual). `created_by_role` ('admin'|'user') menentukan apakah pelatihan auto-approve atau butuh review.

### D2 — Auto-approve untuk pelatihan yang dibuat admin
Saat admin membuat pelatihan (`POST /admin/pelatihan`), `created_by_role='admin'` dan status langsung `verifikasi_diterima`. Admin dapat **edit** (Batalkan/Simpan) pelatihan miliknya. Saat **user** membuat (`POST /pelatihan`), `created_by_role='user'`, status `verifikasi_tertunda`. Admin **hanya** dapat Tolak/Terima, **tidak** boleh edit. Tombol Tolak/Terima terkunci setelah verifikasi dilakukan.

### D3 — Enrollment sebagai tabel mandiri
`pelatihan_enrollment`: user mendaftar + unggah bukti transfer. Status: `pending` → admin tinjau → `approved`/`rejected` (+ alasan). Bukti transfer disimpan sebagai object_key di MinIO via `StorageClient` (kategori baru `training-transfer-evidence`, ≤5MB, JPEG/PNG/PDF). Review admin terkunci setelah verifikasi. Notifikasi otomatis (email + in-app).

### D4 — Badge sebagai tabel mandiri
`pelatihan_badge`: user mengajukan sertifikat; admin verifikasi. Status: `pending` → `approved`/`rejected`. Sertifikat disimpan sebagai object_key di MinIO (kategori baru `training-certificate`, ≤10MB, JPEG/PNG/PDF). `approved_at` dicatat saat disetujui.

### D5 — StorageClient kategori baru
Tiga kategori baru:
- `iklan-suspension-evidence` (≤5MB, JPEG/PNG/PDF) — dikoordinasikan dengan `extend-iklan-moderation`
- `training-transfer-evidence` (≤5MB, JPEG/PNG/PDF) — bukti transfer
- `training-certificate` (≤10MB, JPEG/PNG/PDF) — sertifikat

### D6 — Perbaikan insert field + jumlah_peserta
`lokasi`, `harga`, `tanggal_mulai`, `tanggal_selesai` kini ikut di-INSERT. `jumlah_peserta` adalah field baru untuk kapasitas pelatihan.

### D7 — Kepatuhan standar Phase 1.5
Envelope, error RFC 9457-inspired, IDOR→404, propagasi `request_id`.

### D8 — Params Struct Pattern (Zero `too_many_arguments`)
Fungsi `create()` dan `update_pelatihan()` di repository trait memiliki 7-11 parameter. Alih-alih menggunakan `#[allow(clippy::too_many_arguments)]`, parameter dikelompokkan dalam struct:

- `CreatePelatihanParams<'a>` — untuk `IklanPelatihanRepository::create()`
- `UpdatePelatihanParams<'a>` — untuk `IklanPelatihanRepository::update_pelatihan()`
- `NotifyPayload<'a>` / `NotifyEmail<'a>` — untuk `send_notification()` helper

Pattern yang sama diterapkan ke `iklan-pekerja-service` (`CreatePekerjaParams`), `iklan-barang-bekas-service` (`CreateBarangBekasParams`), dan `iklan-pekerjaan-service` (`CreatePekerjaanParams`). Nol annotation `#[allow(clippy::too_many_arguments)]` di seluruh codebase.

### D9 — Zero Hardcoded String Literals
Semua nama status, role, enrollment status, dan storage category yang sebelumnya hardcoded sebagai string literal kini disentralisasi menjadi named constants:

- `entity::status_name` — 7 konstanta untuk PelatihanStatus
- `entity::role_name` — `USER`, `ADMIN`
- `entity::enrollment_status_name` — `PENDING`, `IN_REVIEW`, `REJECTED`, `APPROVED`
- `service::storage_category` — `SUSPENSION_EVIDENCE`, `TRAINING_TRANSFER`, `TRAINING_CERTIFICATE`
- `repository::DEFAULT_LIMIT` / `repository::CSV_MAX` — pagination defaults

### D10 — Unified ListParams
Empat struct identik (`PelatihanListParams`, `EnrollmentListParams`, `BadgeListParams`, `AdminListParams`) disatukan menjadi `ListParams` generic dengan field `filter_column` + `filter_value`. Type alias untuk backward-compat.

## Risks / Trade-offs

- **Enrollment dan badge punya prasyarat mobile** — endpoint `POST /pelatihan/{id}/enroll` dan `POST /pelatihan/{id}/badge` dibuat dari sisi user; perlu koordinasi dengan tim Flutter.
- **Status pelatihan vs `moderation_status` extend-iklan** — jika extend-iklan menambah `moderation_status` generik, pelatihan akan punya dua kolom status: koordinasikan agar tidak bentrok; idealnya lifecycle 7-status menggantikan flag `is_active` untuk pelatihan, sementara suspend tetap pakai `moderation_status` dari extend-iklan.
- **Transisi otomatis tanggal** — `pelatihan_belum_dimulai` → `pelatihan_berjalan` → `pelatihan_selesai` berdasarkan tanggal: bisa otomatis via scheduled job/cron atau manual; default manual untuk iterasi awal.
- **Volume data enrollment** — pertimbangan paginasi server-side & index `user_id`/`pelatihan_id`.

## Migration Plan

1. Perluas tabel pelatihan: tambah `status` (CHECK 7 nilai), `created_by_role`, `jumlah_peserta`, `reviewed_by`, `review_note`; perbaiki INSERT field.
2. Buat tabel `pelatihan_enrollment` + `pelatihan_badge`.
3. Tambah UNIQUE constraint `(pelatihan_id, user_id)` pada enrollment & badge untuk mencegah duplikasi.
4. Tambah kategori `training-transfer-evidence` & `training-certificate` di `StorageClient`.
5. Implementasi service & handler: create admin (auto-approve) vs user (review), enrollment + review, badge + review.
6. Implementasi listing admin (search/sort/CSV) untuk ketiga sub-halaman.
7. Endpoint sisi user: `POST /pelatihan/{id}/enroll`, `POST /pelatihan/{id}/badge`.
8. Wire di `rejki-app` dengan `require_admin`, `StorageClient`, `NotificationClient`.
9. Swagger mirror DTOs di `rejki-app/src/openapi.rs`.
10. Rollback: migrasi turun drop tabel/kolom baru.

## Open Questions

- **Transisi otomatis tanggal** (belum_dimulai → berjalan → selesai): manual atau cron? Default: manual pada iterasi pertama; otomatis bisa ditambahkan.
- **Koordinasi dengan extend-iklan-moderation**: mana yang punya kolom `moderation_status` pada pelatihan? Usulan: extend-iklan yang menambah (pola seragam), proposal ini menambah 7-status lifecycle yang independen.
- **Bukti transfer per pendaftaran** — cukup 1 file, atau perlu multi? Default: 1 file.
