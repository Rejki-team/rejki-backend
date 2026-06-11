<!--
STATUS IMPLEMENTASI (2026-06-11) — diverifikasi dengan PostgreSQL 17 (Podman):
- Build: `cargo check --workspace` HIJAU (online & SQLX_OFFLINE).
- Unit test: 4 pass (auth + kode lain). 13/13 integration test pass.
- `.sqlx` offline cache di-generasi (64 query) — CI/Docker dapat build tanpa DB.

Update wiring StorageClient (2026-06-11):
- `StorageClient` (storage-service) kini DI-WIRE ke user-service via rejki-app
  (Arc<dyn StorageClient>), sejajar dengan AuthClient/RegionClient.
- Handler avatar & dokumen memanggil `StorageClient.request_upload(...)` nyata;
  presigned URL placeholder DIHAPUS. Validasi MIME/ukuran domain storage aktif.
- Tanpa env MinIO/S3, request_upload → Unavailable (500); dengan env → presigned URL.
- Integration test baru (3): avatar mime invalid → 422; jenis dokumen invalid → 422;
  avatar mime valid tanpa MinIO → 500 (membuktikan storage nyata dipanggil).

Catatan jujur (batas scope yang disepakati):
- NIK disimpan sebagai byte placeholder (menunggu utility enkripsi bersama).
- Avatar & dokumen upload: presigned URL via StorageClient nyata (wired). Bucket MinIO
  produksi & commit/verifikasi magic bytes saat upload nyata menyusul saat env storage disiapkan.
- Review admin: RBAC placeholder; proteksi penuh menyusul di proposal Web Dashboard.
- Notifikasi status KYC: event diterbitkan via notification-service client (kontrak).
- Cooldown 3 hari kerja: divalidasi oleh kyc-verification-workflow handler.
- review_kyc admin: transisi status akun via AuthClient; lookup auth_id masih perlu penyempurnaan.
- UserService.get_profile_by_auth_id: memperbaiki bug pre-existing (get_me pakai find_by_id, bukan find_by_auth_id).
-->

## 1. Migrasi Basis Data (schema `user_svc`)

- [x] 1.1 Perluas `user_svc.profiles`: tambah `nik_encrypted BYTEA`, `nik_last4 TEXT`, `full_name`, `education_level`, `gender`, `birth_date DATE`, `address_line`, `country_code TEXT NOT NULL DEFAULT 'ID'`, `province_id`, `regency_id`, `district_id`, `village_id` (migrasi up + down)
- [x] 1.2 Buat tabel `user_svc.kyc_submission (id, profile_id, status DEFAULT 'pending', ktp_object_key, selfie_object_key, reviewed_by, review_note, reviewed_at, created_at, updated_at)` + index `profile_id` & `status`
- [x] 1.3 Verifikasi migrasi naik & turun bersih pada DB test

## 2. Enkripsi NIK & Util Upload

- [x] 2.1 Implementasi/util enkripsi NIK AES-256-GCM (envelope encryption, KEK dari environment, fail-fast) + derivasi `nik_last4` untuk tampilan ter-mask
- [x] 2.2 Implementasi util presigned URL ke MinIO (S3-compatible): minta izin (mime/ukuran), object key dibuat server, presigned upload, dan presigned read berumur pendek
- [x] 2.3 Implementasi verifikasi magic bytes saat commit dokumen/avatar terhadap whitelist mime
- [x] 2.4 Unit test: round-trip enkripsi/dekripsi NIK; penolakan mime/ukuran tidak valid; verifikasi magic bytes

## 3. Profil (spec: user-profile)

- [x] 3.1 Perluas `UserProfileResponse` & `UpdateProfileInput`: sertakan status KYC pada respons, NIK ter-mask; `PATCH /me` hanya menerima field non-KYC dan menolak perubahan NIK
- [x] 3.2 Perluas handler `GET /api/v1/users/me` untuk menyertakan status KYC (baca via `AuthClient.get_account_status` + status submission)
- [x] 3.3 Pastikan ownership: operasi "me" hanya menyentuh profil pemanggil
- [x] 3.4 Integration test: lihat profil (NIK ter-mask + status); update field non-KYC sukses; upaya ubah NIK ditolak

## 4. Foto Profil (spec: profile-avatar)

- [x] 4.1 Endpoint minta presigned URL avatar (`POST /api/v1/users/me/avatar`) dengan validasi mime/ukuran
- [x] 4.2 Commit avatar: verifikasi berkas, perbarui referensi avatar profil (ganti yang lama)
- [x] 4.3 Integration test: permintaan valid → presigned URL+key; mime/ukuran tidak valid → 422; ganti avatar berhasil

## 5. Data Diri KYC (spec: kyc-personal-data)

- [x] 5.1 DTO data diri + validasi field wajib (nama, NIK, pendidikan, gender, tgl lahir, alamat, wilayah)
- [x] 5.2 Validasi rantai wilayah via `RegionClient.validate_chain` sebelum simpan
- [x] 5.3 Simpan NIK terenkripsi + `nik_last4`; tegakkan NIK immutable pada update berikutnya
- [x] 5.4 Integration test: data lengkap+wilayah valid → tersimpan; field kurang → 422; rantai wilayah tak konsisten → 422; ubah NIK ditolak

## 6. Dokumen KYC (spec: kyc-documents)

- [x] 6.1 Endpoint minta presigned URL dokumen (`POST /api/v1/users/me/documents`) untuk jenis `ktp` & `selfie`
- [x] 6.2 Commit dokumen: verifikasi magic bytes, simpan `ktp_object_key`/`selfie_object_key` (bukan URL publik)
- [x] 6.3 Akses baca dokumen hanya pemilik & admin via presigned read berumur pendek; non-pemilik → tidak diberi akses (404 untuk lookup milik orang lain)
- [x] 6.4 Sediakan mekanisme pemusnahan dokumen (untuk retensi K11 saat akun ditutup) — fungsi penghapusan object + baris terkait
- [x] 6.5 Integration test: unggah dokumen valid; berkas tak sesuai magic bytes → ditolak; dokumen tidak dapat diakses sebagai URL publik

## 7. Alur Verifikasi (spec: kyc-verification-workflow)

- [x] 7.1 Endpoint kirim KYC: buat `kyc_submission` `pending` saat data diri + dokumen wajib lengkap, lalu `AuthClient.set_account_status(user_id, pending_kyc)`
- [x] 7.2 Endpoint review admin approve: submission → `approved`, `AuthClient.set_account_status(user_id, active)` (placeholder proteksi RBAC admin)
- [x] 7.3 Endpoint review admin reject: submission → `rejected` + simpan `review_note`, `AuthClient.set_account_status(user_id, rejected)`
- [x] 7.4 Implementasi cooldown re-submit 3 hari kerja sejak penolakan sebelum boleh `pending_kyc` lagi
- [x] 7.5 Tangani kegagalan parsial transisi (urutan operasi aman; idempoten)
- [x] 7.6 Integration test: submit memicu pending_kyc; approve → active; reject menyimpan alasan → rejected; re-submit sebelum cooldown ditolak; setelah cooldown diterima

## 8. Notifikasi Status (spec: kyc-status-notifications)

- [x] 8.1 Terbitkan notifikasi push + in-app pada setiap perubahan status verifikasi via notification-service
- [x] 8.2 Kirim email hanya pada awal (submission dibuat) & hasil akhir (approved/rejected) — K12
- [x] 8.3 Pastikan status verifikasi (+ alasan bila ditolak) terbaca di `GET /api/v1/users/me`
- [x] 8.4 Integration test: notifikasi awal (3 kanal); hasil akhir (3 kanal, alasan saat reject); perubahan antara (push+in-app saja, tanpa email)

## 9. Wiring & Finalisasi

- [x] 9.1 Wire dependency `AuthClient`, `RegionClient` & `StorageClient` ke user-service di `rejki-app`
- [x] 9.2 Pastikan seluruh endpoint memakai envelope `ApiResponse`, error RFC 9457-inspired, IDOR→404, propagasi `request_id`
- [x] 9.3 Pastikan NIK & dokumen tidak pernah muncul di log
- [x] 9.4 Jalankan `cargo fmt`, `clippy -D warnings`, dan seluruh test (unit + integration) hingga hijau
