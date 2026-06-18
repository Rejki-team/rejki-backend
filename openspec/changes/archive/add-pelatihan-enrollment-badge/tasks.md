## 1. Migrasi Basis Data (schema pelatihan)

- [x] 1.1 Perluas tabel pelatihan: tambah `status TEXT NOT NULL DEFAULT 'verifikasi_tertunda'` (CHECK 7 nilai), `created_by_role TEXT NOT NULL DEFAULT 'user'` (CHECK 'admin'|'user'), `jumlah_peserta INTEGER` (up + down)
- [x] 1.2 Perbaiki INSERT: pastikan `lokasi`, `harga`, `tanggal_mulai`, `tanggal_selesai` ikut tersimpan di `repo.create()`
- [x] 1.3 Buat tabel `pelatihan_enrollment (id UUID PK, pelatihan_id UUID, user_id UUID, bukti_transfer_object_key TEXT, status TEXT DEFAULT 'pending' CHECK pending|in_review|rejected|approved, reviewed_by UUID, review_note TEXT, created_at TIMESTAMPTZ, updated_at TIMESTAMPTZ)` + index `pelatihan_id`, `user_id`, `status` (up + down)
- [x] 1.4 Buat tabel `pelatihan_badge (id UUID PK, pelatihan_id UUID, user_id UUID, sertifikat_object_key TEXT, approved_at TIMESTAMPTZ, status TEXT DEFAULT 'pending' CHECK pending|in_review|rejected|approved, reviewed_by UUID, review_note TEXT, created_at TIMESTAMPTZ, updated_at TIMESTAMPTZ)` + index `pelatihan_id`, `user_id`, `status` (up + down)
- [x] 1.5 Tambah `reviewed_by UUID`, `review_note TEXT` ke tabel pelatihan + UNIQUE constraint `(pelatihan_id, user_id)` pada enrollment & badge (migration 20260615000001)
- [x] 1.6 Verifikasi migrasi naik & turun bersih pada DB test

## 2. Kategori StorageClient Baru

- [x] 2.1 Tambah kategori `training-transfer-evidence` (≤5MB, JPEG/PNG/PDF) di `StorageClient`
- [x] 2.2 Tambah kategori `training-certificate` (≤10MB, JPEG/PNG/PDF) di `StorageClient`
- [x] 2.3 Unit test: penolakan mime/ukuran tidak valid

## 3. Status & Auto-Approve Pelatihan (spec: pelatihan-moderation)

- [x] 3.1 Definisikan enum `PelatihanStatus` 7 nilai + `status_name` module; terapkan di entity & kolom DB
- [x] 3.2 Definisikan `CreatedByRole` enum + `role_name` module ('admin'|'user') pada entity; set saat create
- [x] 3.3 Pelatihan dibuat admin (`POST /admin/pelatihan`): auto-approve → status `verifikasi_diterima`
- [x] 3.4 Pelatihan dibuat user (POST existing): status `verifikasi_tertunda`, menunggu review
- [x] 3.5 Admin dapat **edit** (PATCH) pelatihan miliknya (`created_by_role='admin'`, only owner)
- [x] 3.6 Admin dapat **cancel** (DELETE soft) pelatihan miliknya (`created_by_role='admin'`)
- [x] 3.7 Admin **tidak** dapat edit pelatihan milik user (FR-ADM-TRN-05)
- [x] 3.8 Endpoint review admin `POST /admin/pelatihan/{id}/review` {approved:bool, review_note?}: approve → `verifikasi_diterima`, reject → `verifikasi_ditolak`
- [x] 3.9 Tombol Tolak/Terima **terkunci** setelah verifikasi dilakukan (FR-ADM-TRN-06)
- [x] 3.10 Notifikasi email + in-app ke pengaju via `NotificationClient`
- [x] 3.11 Integration test: alur auto-approve admin, review user, edit/cancel admin, tolak wajib alasan

## 4. Konfirmasi Pelatihan / Enrollment (spec: pelatihan-enrollment)

- [x] 4.1 Endpoint user `POST /api/v1/pelatihan/{id}/enroll` (auth user): minta presigned upload bukti transfer + simpan enrollment `pending`
- [x] 4.2 Endpoint commit bukti: verifikasi magic bytes, simpan `bukti_transfer_object_key`
- [x] 4.3 Listing admin `GET /admin/enrollments` (proteksi `require_admin`) — search (judul/kode/penyelenggara), sort status, pagination
- [x] 4.4 Endpoint `GET /admin/enrollments/{id}` (detail + foto bukti)
- [x] 4.5 Endpoint `POST /admin/enrollments/{id}/review` {approved, review_note?}: approve→approved, reject→rejected (+alasan wajib)
- [x] 4.6 Tombol Tolak/Terima terkunci setelah verifikasi (FR-ADM-CNF-02)
- [x] 4.7 Notifikasi email + in-app ke pendaftar
- [x] 4.8 Export CSV
- [x] 4.9 Integration test: alur pendaftaran + unggah bukti + review admin + tolak wajib alasan

## 5. Badge / Sertifikat (spec: pelatihan-badge)

- [x] 5.1 Endpoint user `POST /api/v1/pelatihan/{id}/badge` (auth user): minta presigned upload sertifikat + simpan badge `pending`
- [x] 5.2 Endpoint commit sertifikat: verifikasi magic bytes, simpan `sertifikat_object_key`
- [x] 5.3 Listing admin `GET /admin/badges` (proteksi `require_admin`) — search, sort, pagination
- [x] 5.4 Endpoint `GET /admin/badges/{id}` (detail + sertifikat)
- [x] 5.5 Endpoint `POST /admin/badges/{id}/review` {approved, review_note?}: approve → `approved` + `approved_at`, reject → `rejected` + alasan
- [x] 5.6 Tombol Tolak/Terima terkunci setelah verifikasi (FR-ADM-BDG-02)
- [x] 5.7 Notifikasi email + in-app ke pengaju
- [x] 5.8 Export CSV
- [x] 5.9 Integration test: alur pengajuan sertifikat + review admin

## 6. Listing Admin (spec: pelatihan-admin-listing)

- [x] 6.1 DTO pelatihan admin dengan kolom sesuai User Story (ID, Judul, Deskripsi, Penyelenggara, Lokasi, Tanggal, Jumlah Peserta, Status)
- [x] 6.2 Search (judul/kode/penyelenggara), filter by status, pagination `limit/offset`
- [x] 6.3 Admin read-only pada data pelatihan user; hanya aksi Tolak/Terima & suspend
- [x] 6.4 Integration test: search/filter/sort mengembalikan subset benar

## 7. Wiring & Finalisasi

- [x] 7.1 Wire `require_admin`, `StorageClient`, `NotificationClient` ke router pelatihan di `rejki-app`
- [x] 7.2 Pastikan envelope `ApiResponse`, error RFC 9457-inspired, IDOR→404, propagasi `request_id`
- [x] 7.3 Pastikan bukti transfer dan sertifikat tidak divalidasi masa pakainya (hanya magic bytes & mime)
- [x] 7.4 Swagger/OpenAPI mirror DTOs di `rejki-app/src/openapi.rs` — 18 endpoint stubs + 10 DTO types + tags `pelatihan` & `admin-pelatihan`
- [x] 7.5 `cargo fmt`, `clippy -D warnings`, seluruh test hijau — zero warnings, zero errors

## 8. Code Quality & Refactoring (pasca-implementasi)

- [x] 8.1 Zero Hardcoded (#16): tambah `status_name`, `role_name`, `enrollment_status_name`, `storage_category` constants; `DEFAULT_LIMIT`/`CSV_MAX`
- [x] 8.2 Struct Optimization (#15): unify `PelatihanListParams`/`EnrollmentListParams`/`BadgeListParams` → `ListParams` generic
- [x] 8.3 Zero God Function (#17): extract `sanitize()`, `validate_reject_note()`, `request_storage_upload()`, `send_notification()`, `notify_pelatihan_review()`, `notify_review()` helpers
- [x] 8.4 Zero God Class (#18): single responsibility verified — no struct handles >1 concern
- [x] 8.5 Zero `#[allow(clippy::too_many_arguments)]`: `CreatePelatihanParams`/`UpdatePelatihanParams`/`NotifyPayload` structs — diterapkan ke 4 service
- [x] 8.6 CSV formula injection prevention (CWE-1236): escape `=`, `+`, `-`, `@` prefix di `escape_csv()`
- [x] 8.7 XSS: `ammonia::clean_text()` diterapkan ke `penyelenggara` di semua create/update path
- [x] 8.8 Duplicate prevention: UNIQUE constraint pada enrollment & badge
- [x] 8.9 Code review report: `CODE_REVIEW.md` — 3 putaran, 10 temuan, 30 poin compliance
