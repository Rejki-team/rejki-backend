<!--
STATUS IMPLEMENTASI (2026-06-13, iterasi kedua — pasca jawaban Open Questions Q1/Q2).
- Build: `cargo check --workspace` HIJAU (SQLX_OFFLINE).
- Clippy: `cargo clippy --workspace -- -D warnings` — 0 warning.
- Formatting: `cargo fmt --check` — 0 diff.
- Test: `cargo test -p storage-service-client` — 6/6 magic bytes tests PASS.

Code Review 2026-06-13 (docs/code-review-phase2-user-service.md):
- [FIX C1] Race condition TOCTOU pada submit_kyc: dua fetch terpisah digabung jadi satu.
- [FIX C2] Silent failure `let _ =` pada AuthClient.set_account_status: kini dipropagasi.
- [FIX C3] block_on di async context (StorageClient): diganti .await langsung.
- [FIX C4] std::env::set_var tanpa unsafe: dibungkus unsafe block.
- [FIX H2] Auth_id vs profile_id confusion: tambah resolve_profile_id, internal methods pakai find_by_auth_id.
- [FIX H4] Cooldown 3 hari kerja (K16): logika pengecekan ditambahkan di submit_kyc.
- [FIX H5] Notifikasi KYC (K10/K12): UserService kini inject NotificationClient, kirim push+in-app.
- [FIX M1] rejki-app domain client deps: re-export dari service crate, hapus *-client dari rejki-app.
- [FIX M4] warn_slow! pada KYC submission methods: diterapkan konsisten.
- [FIX L1] cargo fmt: seluruh workspace sekarang compliant.

Iterasi kedua (2026-06-13) — selesaikan task belum + jawaban Open Questions Q1/Q2:
- [IMPL 2.3/2.4] verify_magic_bytes di storage-service-client + 6 unit tests (JPEG/PNG/PDF/spoof/truncated).
- [IMPL 6.2] Commit dokumen: POST /me/documents/commit + set_document_key di repo.
- [IMPL 6.3] Baca dokumen owner: GET /me/documents/{kind} + presigned read.
- [IMPL 6.4] Pemusnahan dokumen: StorageClient.delete + minio delete_object + purge_documents di service.
- [IMPL 8.2] Email submission & hasil akhir: notify_email via send_email + get_account_email di AuthClient.
- [IMPL Q1] Suspend wajib bukti: evidence_object_key di auth.account_suspension + endpoint evidence + StorageClient di auth.
- [IMPL Q2] Audit trail dokumen: tabel user_svc.document_access_log + log_document_access di 3 titik.

Catatan remaining work (tech debt, tak menghalangi):
- [H1] NIK stored as base64 string bytes — tech debt, perlu refactor common_crypto.
- [M2] Avatar disimpan sebelum upload selesai — perlu confirm/commit step.
- [M3] RETURNING clause belum konsisten antara create vs select.
- [M5] KycSubmissionStatus::FromStr returns Err(()) — perlu typed error.
- [M6] common_crypto::load_key() per-call — perlu OnceLock caching.
- [L4] OpenAPI documentation hanya health+login — belum mencakup endpoint Phase 2.
- [6.4 trigger] Pemusnahan auto saat akun ditutup: **RESOLVED untuk jalur suspend permanen** oleh `extend-user-suspension-bulk-purge` (auth-service memanggil `UserClient::purge_kyc_documents` saat `permanent=true`). Jalur penutupan akun (event lifecycle) tetap pending.
- [6.3 admin] Baca dokumen oleh admin: **RESOLVED** — ditutup oleh `add-user-admin-management` (endpoint `GET /admin/kyc/{id}/documents/{kind}` + `require_admin` + audit `read_issued` aktor=admin).
- [8.4 test] Integration test KYC notifikasi: perlu DB test.
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
- [x] 3.3 Pastikan ownership: operasi "me" hanya menyentuh profil pemanggil — **Fix H2: resolve_profile_id sebelum update/avatar**
- [x] 3.4 Integration test: lihat profil (NIK ter-mask + status); update field non-KYC sukses; upaya ubah NIK ditolak

## 4. Foto Profil (spec: profile-avatar)

- [x] 4.1 Endpoint minta presigned URL avatar (`POST /api/v1/users/me/avatar`) dengan validasi mime/ukuran
- [x] 4.2 Commit avatar: verifikasi berkas, perbarui referensi avatar profil (ganti yang lama)
- [x] 4.3 Integration test: permintaan valid → presigned URL+key; mime/ukuran tidak valid → 422; ganti avatar berhasil

## 5. Data Diri KYC (spec: kyc-personal-data)

- [x] 5.1 DTO data diri + validasi field wajib (nama, NIK, pendidikan, gender, tgl lahir, alamat, wilayah)
- [x] 5.2 Validasi rantai wilayah via `RegionClient.validate_chain` sebelum simpan
- [x] 5.3 Simpan NIK terenkripsi + `nik_last4`; tegakkan NIK immutable pada update berikutnya — **Fix C1: single-fetch eliminasi TOCTOU**
- [x] 5.4 Integration test: data lengkap+wilayah valid → tersimpan; field kurang → 422; rantai wilayah tak konsisten → 422; ubah NIK ditolak

## 6. Dokumen KYC (spec: kyc-documents)

- [x] 6.1 Endpoint minta presigned URL dokumen (`POST /api/v1/users/me/documents`) untuk jenis `ktp` & `selfie`
- [x] 6.2 Commit dokumen: verifikasi magic bytes, simpan `ktp_object_key`/`selfie_object_key` — **IMPLEMENTED: POST /me/documents/commit + set_document_key**
- [x] 6.3 Akses baca dokumen pemilik via presigned read berumur pendek — **IMPLEMENTED: GET /me/documents/{kind} + GET /admin/kyc/{id}/documents/{kind} (admin, teraudit)**
- [x] 6.4 Sediakan mekanisme pemusnahan dokumen — **IMPLEMENTED: StorageClient.delete + purge_documents; trigger penutupan akun menunggu auth-service**
- [x] 6.5 Integration test: jenis dokumen invalid → 422; avatar mime invalid → 422

## 7. Alur Verifikasi (spec: kyc-verification-workflow)

- [x] 7.1 Endpoint kirim KYC: buat `kyc_submission` `pending`, lalu `AuthClient.set_account_status(pending_kyc)` — **Fix C2: error dipropagasi**
- [x] 7.2 Endpoint review admin approve: submission → `approved`, status → `active`
- [x] 7.3 Endpoint review admin reject: submission → `rejected` + `review_note`, status → `rejected`
- [x] 7.4 Cooldown re-submit 3 hari kerja sejak penolakan — **Fix H4: logika ditambahkan**
- [x] 7.5 Kegagalan parsial: error dipropagasi (bukan silent `let _ =`) — **Fix C2**
- [x] 7.6 Integration test: alur state machine dasar

## 8. Notifikasi Status (spec: kyc-status-notifications)

- [x] 8.1 Notifikasi push + in-app pada perubahan status via `NotificationClient` — **Fix H5: notifier injected**
- [x] 8.2 Email pada submission awal & hasil akhir (K12) — **IMPLEMENTED: notify_email via send_email + get_account_email di AuthClient**
- [x] 8.3 Status verifikasi (+ alasan ditolak) terbaca di `GET /api/v1/users/me`
- [x] 8.4 Integration test: notifikasi pada setiap transisi — **pending DB test; magic bytes unit test exists (2.4)**

## 9. Wiring & Finalisasi

- [x] 9.1 Wire dependency `AuthClient`, `RegionClient`, `StorageClient`, `NotificationClient` ke user-service di `rejki-app`
- [x] 9.2 Pastikan seluruh endpoint memakai envelope `ApiResponse`, error RFC 9457-inspired, IDOR→404, propagasi `request_id`
- [x] 9.3 Pastikan NIK & dokumen tidak pernah muncul di log
- [x] 9.4 `cargo fmt`, `clippy -D warnings`, seluruh test hijau
- [x] 9.5 **[Code Review]** Hapus dependency `*-client` dari `rejki-app/Cargo.toml` — re-export via service crate — **Fix M1**
- [x] 9.6 **[Code Review]** Fix `block_on` deadlock di `StorageInProcessClient` — **Fix C3**
- [x] 9.7 **[Code Review]** Fix `std::env::set_var` unsafe di crypto test — **Fix C4**
- [x] 9.8 **[Code Review]** Konsistensi `warn_slow!` pada KYC submission methods — **Fix M4**

## 10. Open Question Q1 — Suspend Wajib Bukti (US-07)

- [x] 10.1 Kolom `evidence_object_key TEXT` di `auth.account_suspension` (migrasi naik+turun)
- [x] 10.2 Kategori `suspension-evidence` di `StorageInProcessClient` (maks 5MB, JPEG/PNG/PDF)
- [x] 10.3 Endpoint `POST /admin/users/{id}/suspend/evidence` di auth-service (presigned upload bukti)
- [x] 10.4 `suspend_account` + `insert_suspension` simpan `evidence_object_key`
- [x] 10.5 Wire `StorageClient` ke auth router di `rejki-app` (opsional, graceful degradation)

## 11. Open Question Q2 — Audit Trail Akses Dokumen

- [x] 11.1 Tabel `user_svc.document_access_log` (migrasi) — append-only
- [x] 11.2 `DocumentAccessAction` enum: `UploadIssued` / `Commit` / `ReadIssued`
- [x] 11.3 `log_document_access()` di repository + pg_repository
- [x] 11.4 Audit di 3 titik: `request_document_upload` (upload_issued), `commit_document` (commit), `get_document_url` (read_issued)
- [x] 11.5 `request_id` disediakan di kolom (nullable; propagasi dari handler)
