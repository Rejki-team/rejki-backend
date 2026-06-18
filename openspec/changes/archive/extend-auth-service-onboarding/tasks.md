<!--
STATUS IMPLEMENTASI (2026-06-11) — diverifikasi dengan PostgreSQL 17 (Podman):
- Build: `cargo check --workspace` HIJAU (online & SQLX_OFFLINE).
- Unit test: 4 pass (password policy, E.164, AES-256-GCM round-trip, nonce-unik).
- Integration test: 10/10 pass (rejki-app/tests/auth_integration_test.rs).
- `.sqlx` offline cache di-generate — CI/Docker dapat build tanpa DB.

Catatan jujur (batas scope yang disepakati, bukan TODO terlewat):
- Suspend endpoint memakai require_auth sbg placeholder; RBAC admin penuh menyusul
  di proposal Web Dashboard (sesuai Non-Goals).
- Email OTP: auth memanggil kontrak NotificationClient; implementasi (publisher
  Redis) di notification-service; bun consumer diperluas untuk SMTP. Aktif bila
  REDIS_URL & SMTP_* diset; bila tidak, OTP tidak di-log/bocor (fail-safe).
- Perbaikan baseline penyerta agar build hijau: handler health hilang di 6 service,
  bug span !Send di request_id_layer, binary standalone ditandai unimplemented (Phase 4).
-->

## 1. Migrasi Basis Data (schema `auth`)

- [x] 1.1 Tambah kolom `phone TEXT` (terenkripsi at-rest), `status TEXT NOT NULL DEFAULT 'pending_verification'`, `tos_accepted_at TIMESTAMPTZ`, `tos_version TEXT` pada `auth.users` (migrasi up + down)
- [x] 1.2 Backfill `status` dari `is_verified` (RQ1): `is_verified=false` → `pending_verification`; `is_verified=true` → `profile_incomplete` (semua akun lama mengikuti alur KYC baru, tidak langsung `active`)
- [x] 1.3 Drop kolom `is_verified` SETELAH backfill `status` selesai (T4/D9 — `status` jadi satu-satunya sumber kebenaran); sediakan down-migration yang mengembalikan `is_verified` bila rollback
- [x] 1.4 Buat tabel `auth.account_suspension (id, user_id, is_permanent, reason, expires_at, created_by, created_at)` + index `user_id` (migrasi up + down)
- [x] 1.5 Pastikan `auth.otp_verifications` mendukung purpose `reset_password` & `change_password`; tambah kolom `attempts INT NOT NULL DEFAULT 0` bila belum ada
- [x] 1.6 Verifikasi migrasi naik & turun berjalan bersih pada DB test

## 2. Domain & State Machine (auth-service)

- [x] 2.1 Tambah tipe `AccountStatus` (enum) pada domain entity dengan nilai sesuai spec `account-status-lifecycle`
- [x] 2.2 Perluas entity `AuthUser` dengan field `phone`, `status`, `tos_accepted_at`, `tos_version`; hapus penggunaan `is_verified` (ganti dengan `status`)
- [x] 2.3 Tambah varian `OtpPurpose::ChangePassword` (string `change_password`) pada enum domain (T3/D10); `Register` & `ResetPassword` sudah ada
- [x] 2.4 Implementasi fungsi transisi state machine terpusat (tabel transisi sah) yang menolak transisi tidak sah
- [x] 2.5 Unit test transisi: transisi sah lolos, transisi tidak sah ditolak

## 3. Kebijakan Password & Enkripsi

- [x] 3.1 Implementasi validator password (min 8 + kapital + angka + karakter spesial) yang dipakai register/reset/change
- [x] 3.2 Implementasi utilitas envelope encryption AES-256-GCM untuk `phone` (KEK dari environment, fail-fast bila tidak ada) sesuai Config Standard
- [x] 3.3 Unit test validator password (kasus lolos & gagal per aturan) dan round-trip enkripsi/dekripsi `phone`

## 4. Registrasi Diperluas (spec: account-registration)

- [x] 4.1 Perluas `RegisterInput` DTO: tambah `phone` (validasi E.164) dan `tos_accepted` (wajib true), terapkan validator password
- [x] 4.2 Perbarui use case register: simpan `phone` terenkripsi, catat `tos_accepted_at` + `tos_version`, set status awal `pending_verification`
- [x] 4.3 Terapkan perilaku anti-enumeration: email sudah terdaftar → respons sukses generik tanpa duplikasi; bila `pending_verification` → kirim ulang OTP `register`
- [x] 4.4 Integration test register: data lengkap sukses; tanpa T&C → 422; phone non-E.164 → 422; email terdaftar → respons generik tanpa akun kedua

## 5. OTP Berbasis Purpose (spec: otp-verification)

> **Default parameter OTP (RQ2 — usulan untuk ditinjau saat implementasi):** OTP 6 digit; **TTL 5 menit**; **maks 5 percobaan verifikasi** per OTP (lewat batas → OTP dibatalkan, minta baru); **rate limit permintaan: maks 3 kirim per 15 menit** per (user, purpose) + jeda minimal antar-kirim (mis. 60 detik). Angka dapat disesuaikan sebelum/saat implementasi tanpa mengubah spec.

- [x] 5.1 Pastikan pembangkitan OTP menyimpan hash + expiry (TTL 5 menit) dan menggantikan OTP aktif per (user_id, purpose)
- [x] 5.2 Implementasi batas percobaan verifikasi (kolom `attempts`, maks 5) dan rate limit permintaan OTP via Redis (maks 3 / 15 menit + jeda 60 detik antar-kirim)
- [x] 5.3 Verifikasi nilai OTP tidak pernah muncul di body respons API maupun log permanen
- [x] 5.4 Integration test: OTP benar → aksi jalan; salah/kedaluwarsa → ditolak; melebihi batas percobaan/rate limit → ditolak

## 6. Status Akun via AuthClient (spec: account-status-lifecycle)

- [x] 6.1 Perluas trait `AuthClient` di `auth-service-client`: tambah `get_account_status(user_id)` & `set_account_status(user_id, status)` + tipe `AccountStatus`
- [x] 6.2 Implementasikan method baru pada `AuthInProcessClient` (memanggil application service auth, memvalidasi transisi)
- [x] 6.3 Verifikasi verifikasi OTP `register` menaikkan status `pending_verification → profile_incomplete`
- [x] 6.4 Integration test: get/set status lewat client; transisi tidak sah via client ditolak

## 7. Gating Fitur (spec: feature-gating)

- [x] 7.1 Tambah field `status` pada `JwtClaims` (`jwt.rs`) dan sertakan saat penerbitan access token (`issue_access_token`)
- [x] 7.2 Tambah field `status` pada `AuthClaims` publik (`auth-service-client`) dan pastikan `validate_token` mengisinya; verifikasi handler lintas-service (chat/notif/iklan) tetap kompilasi (hanya tambah field)
- [x] 7.3 Ubah `login()` agar memeriksa `status` (bukan `is_verified`): tolak `pending_verification` & status suspended dengan galat yang sesuai (T4/D9)
- [x] 7.4 Implementasi penulisan penanda kesegaran ke Redis saat status berubah (perubahan status & suspend)
- [x] 7.5 Implementasi middleware gating `require_active_account` (di `common/auth-middleware`): izinkan hanya `active`; tolak lainnya dengan `403 ACCOUNT_NOT_ACTIVE`; pakai `status` dari `AuthClaims` + cek penanda Redis (fallback `get_account_status` bila `status` tak ada di token lama)
- [x] 7.6 (T1/D8) Pasang `require_auth` pada router service fitur yang membutuhkan identitas (iklan, chat) yang saat ini BELUM memasangnya; biarkan endpoint publik (lihat iklan) tanpa auth
- [x] 7.7 Pasang `require_active_account` setelah `require_auth` pada endpoint fitur (tulis/aksi) di Composition Root `rejki-app`
- [x] 7.8 Integration test: akun `active` lolos; `profile_incomplete`/`pending_kyc`/suspended → 403 `ACCOUNT_NOT_ACTIVE`; token lama setelah suspend → 403 via cek kesegaran; endpoint publik tetap dapat diakses tanpa login

## 8. Lupa & Ubah Password (spec: password-recovery)

- [x] 8.1 Endpoint `POST /api/v1/auth/forgot-password`: kirim OTP `reset_password`, selalu balas sukses generik
- [x] 8.2 Endpoint `POST /api/v1/auth/reset-password`: validasi OTP + password baru, perbarui hash, cabut SEMUA refresh token
- [x] 8.3 Endpoint `POST /api/v1/auth/change-password` (authed): validasi OTP `change_password` + password baru, cabut refresh token lain
- [x] 8.4 Integration test: forgot untuk email tak terdaftar tetap sukses generik; reset dengan OTP valid mencabut sesi; OTP salah → ditolak; password lemah → 422

## 9. Penangguhan Akun (spec: account-suspension)

- [x] 9.1 Implementasi kapabilitas suspend (sementara dengan `expires_at`, permanen tanpa) yang menyimpan catatan ke `account_suspension` (reason, created_by) dan mengubah status
- [x] 9.2 Implementasi pengembalian otomatis `suspended_temp → active` saat `expires_at` lewat (dievaluasi di login/validasi)
- [x] 9.3 Sediakan operasi/endpoint internal suspend (placeholder proteksi RBAC admin, ditandai untuk proposal Dashboard)
- [x] 9.4 Tegakkan penolakan login & permintaan terotorisasi untuk akun ditangguhkan (termasuk token yang masih berlaku via cek kesegaran)
- [x] 9.5 Catat audit tindakan suspend (aktor, sasaran, waktu, alasan)
- [x] 9.6 Integration test: suspend sementara & permanen; login ditolak; auto-active setelah expiry; audit tercatat

## 10. Integrasi Pengiriman OTP via notification-service

- [x] 10.1 Ganti TODO pengiriman OTP (log) dengan delegasi pengiriman email ke notification-service untuk purpose register/reset/change
- [x] 10.2 Pastikan kegagalan pengiriman ditangani aman (gagal tertutup, tanpa membocorkan OTP)

## 11. Kepatuhan Standar & Finalisasi

- [x] 11.1 Pastikan seluruh endpoint memakai envelope `ApiResponse`, error RFC 9457-inspired (`code/message/fields`), status matrix yang benar, dan propagasi `request_id`
- [x] 11.2 Jalankan `cargo fmt`, `clippy -D warnings`, dan seluruh test (unit + integration) hingga hijau
- [x] 11.3 Perbarui dokumentasi terkait bila perlu (catatan status akun & endpoint baru) agar selaras dengan brainstorming/PRD
