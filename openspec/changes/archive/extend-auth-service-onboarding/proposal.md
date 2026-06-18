## Why

Alur autentikasi Rejki saat ini hanya mendukung registrasi minimal (email + password), verifikasi OTP, dan sesi JWT — belum cukup untuk onboarding Phase 2 yang mewajibkan kelengkapan data diri (KYC), penguncian fitur sampai akun aktif, pemulihan/penggantian password, dan penangguhan akun oleh admin. Tanpa fondasi ini, user-service (KYC) dan region-service belum bisa dibangun karena keduanya bergantung pada status akun yang dimiliki auth-service. Proposal ini memperluas auth-service sebagai fondasi onboarding, mengikuti keputusan brainstorming K1–K16 dan standar Phase 1.5.

## What Changes

- **Registrasi diperluas** (US-01): tambah field `phone` (format E.164, disimpan tanpa verifikasi SMS — K8) dan persetujuan syarat & ketentuan wajib (`tos_accepted`) yang dicatat (`tos_accepted_at`, `tos_version`). Registrasi menerapkan respons anti-enumeration (tidak membocorkan apakah email sudah terdaftar).
- **Kebijakan password diperketat** (K4) — **BREAKING** terhadap validasi lama: minimal 8 karakter dengan kombinasi minimal 1 huruf kapital, 1 angka, dan 1 karakter spesial. Berlaku pada register, reset, dan change password.
- **State machine status akun** (US-04): kolom `status` pada `auth.users` menggantikan ketergantungan tunggal pada `is_verified`, dengan nilai `pending_verification → profile_incomplete → pending_kyc → (rejected | active) → (suspended_temp | suspended_permanent)`. Hanya `active` yang boleh memakai fitur.
- **Penegakan gating** (K1): status di-embed dalam klaim JWT (menambah field `status` pada `JwtClaims` dan `AuthClaims`) untuk kecepatan; untuk hal sensitif (suspend, perubahan status) kesegaran dijamin via cek cepat ke Redis. Endpoint fitur menolak token non-`active` dengan `403` + kode mesin `ACCOUNT_NOT_ACTIVE`. **Mencakup pemasangan `require_auth` + gating ke router service fitur** (iklan/chat) yang saat ini belum memasang middleware autentikasi.
- **Migrasi penuh ke `status`** — **BREAKING** internal: `is_verified` dipensiunkan; `login()` dan seluruh cek kelayakan berpindah ke `status`. Kolom `is_verified` di-drop setelah backfill.
- **Perluasan trait `AuthClient`** (K2) — **BREAKING** terhadap kontrak client lama: tambah `set_account_status(user_id, status)` dan `get_account_status(user_id)`, dipanggil in-process oleh user-service/admin saat KYC approve/reject & suspend. Status akun tetap milik auth-service (aliran satu arah user/admin → auth).
- **Lupa password** (US-05): endpoint `forgot-password` (kirim OTP ke email, selalu balas sukses) dan `reset-password` (OTP + password baru, cabut semua refresh token).
- **Ubah password** (US-06): endpoint `change-password` (terautentikasi + OTP, cabut refresh token lain).
- **Suspend akun** (US-07): kapabilitas suspend sementara (dengan `expires_at`, otomatis kembali `active` setelah lewat) dan permanen, dengan `reason` & `created_by`, ditegakkan saat login & validasi token, dengan dasar audit log.
- **OTP** (K15): tetap memakai tabel `otp_verifications` yang ada dengan `purpose` berbeda (`register`, `reset_password`, `change_password`) — tanpa tabel/mekanisme baru; OTP di-hash, TTL pendek, rate limit & batas percobaan.
- **Pengiriman OTP**: alur menyatakan pengiriman OTP email via notification-service (menggantikan TODO yang saat ini hanya mencatat log); mekanik FCM/Bun di luar scope.

## Capabilities

### New Capabilities
- `account-registration`: registrasi pengguna (email, phone, password, persetujuan T&C), anti-enumeration, dan kebijakan password.
- `account-status-lifecycle`: state machine status akun, transisi, kepemilikan status di auth-service, dan perluasan `AuthClient` untuk baca/ubah status.
- `feature-gating`: penegakan akses berbasis status akun (JWT + cek Redis), penolakan `ACCOUNT_NOT_ACTIVE`.
- `password-recovery`: lupa password & ubah password dengan otorisasi OTP dan pencabutan sesi.
- `account-suspension`: suspend sementara/permanen oleh admin, penegakan saat login/validasi token, audit.
- `otp-verification`: pembangkitan, penyimpanan ter-hash, kedaluwarsa, rate limit, dan konsumsi OTP berbasis `purpose` (memformalkan kemampuan yang sebagian sudah ada).

### Modified Capabilities
<!-- Tidak ada: belum ada spec yang ter-arsip di openspec/specs/. Semua kemampuan di atas didefinisikan sebagai kapabilitas baru meski sebagian memperluas kode auth-service yang sudah berjalan. -->

## Impact

- **Kode**: `rust-services/auth-service/` (domain entity `AuthUser` + `OtpPurpose` tambah varian `ChangePassword`, application service & DTO, infrastructure `jwt.rs` (tambah `status` di `JwtClaims`) & repository, interface handlers & router), `rust-services/auth-service-client/` (trait `AuthClient` + tipe `AccountStatus`, tambah `status` di `AuthClaims`), `rust-services/common/auth-middleware/` (middleware gating `require_active_account`), `rust-services/rejki-app/` (wiring `require_auth` + gating ke router service fitur). **Catatan lintas-service**: penambahan field `status` pada `AuthClaims` menyentuh seluruh handler yang memakainya (chat, notif, 4 iklan) — kompatibel karena hanya menambah field.
- **Basis data** (schema `auth`): kolom baru `phone` (terenkripsi), `status`, `tos_accepted_at`, `tos_version` pada `auth.users`; kapabilitas suspend (kolom/tabel `account_suspension` dengan `reason`, `expires_at`, `created_by`); migrasi `otp_verifications` mendukung `purpose` baru. Tidak ada FK lintas-schema.
- **API** (`/api/v1/auth/*`): perubahan body `register`; endpoint baru `forgot-password`, `reset-password`, `change-password`; kapabilitas internal suspend (endpoint kelak diproteksi RBAC admin).
- **Dependensi**: Redis (cek status cepat / revocation), notification-service (pengiriman OTP email), enkripsi AES-256-GCM dengan envelope encryption (KEK di environment, K14).
- **Konsumen hilir**: user-service (KYC) dan region-service akan dibangun di proposal terpisah dan memanggil `AuthClient` yang diperluas ini.
- **Keamanan & standar**: mengikuti envelope `ApiResponse`, error RFC 9457-inspired, prefix `/api/v1/`, snake_case DB, `created_at`/`updated_at`, soft delete bila relevan, propagasi `request_id`.

## Non-Goals

- Data KYC/profil pengguna (NIK, dokumen, alamat) — proposal **user-service** terpisah.
- Data wilayah Indonesia berjenjang — proposal **region-service** terpisah.
- RBAC admin penuh (peran Super Admin/Moderator/Support) — lihat PRD Web Dashboard; proposal ini hanya menyediakan kapabilitas internal suspend yang kelak diproteksi RBAC.
- Verifikasi nomor telepon via OTP SMS (K8: telepon cukup disimpan).
- Mekanik internal pengiriman FCM/Bun consumer (notification-service Phase 2 terpisah).
