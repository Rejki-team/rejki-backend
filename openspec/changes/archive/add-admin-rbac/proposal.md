## Why

Rejki Web Dashboard ([docs/prd/prd-dashboard.md](../../../docs/prd/prd-dashboard.md) v0.2) menuntut pengguna ber-peran **admin** yang login dengan email+password dan mengakses endpoint moderasi. Backend saat ini **tidak memiliki konsep `role`/RBAC** sama sekali: `JwtClaims` (`auth-service/src/infrastructure/jwt.rs`) dan `AuthClaims` (`auth-service-client`) tidak punya field `role`, dan endpoint admin yang sudah ada (`POST /api/v1/users/admin/kyc/{id}/review`, suspend di auth-service) hanya ber-placeholder tanpa proteksi peran. Tanpa fondasi RBAC, seluruh change moderasi dashboard (`extend-iklan-moderation`, `add-pelatihan-enrollment-badge`, `add-content-reports`, `add-corporate-comms`) tidak dapat memproteksi endpointnya. Proposal ini menyediakan fondasi RBAC sesuai User Story (FR-ADM-AUTH-01..05) dan keputusan pemilik produk: **satu role `admin` yang extensible** + **login admin email/password tanpa OTP** (akun di-inject DBA, tanpa self-register).

## What Changes

- **Kolom `role`** (US/FR-ADM-AUTH-02/03): tambah `role TEXT NOT NULL DEFAULT 'user'` pada `auth.users` dengan `CHECK (role IN ('user','admin'))` yang dirancang **extensible** (penambahan nilai enum di masa depan — mis. `super_admin`/`moderator`/`support` — bersifat aditif, bukan breaking).
- **`role` pada klaim JWT** (FR-ADM-AUTH-03): tambah field `role` pada `JwtClaims` dan `AuthClaims` (aditif, kompatibel — handler existing yang memakai `AuthClaims` tidak rusak).
- **Login admin email+password tanpa OTP** (FR-ADM-AUTH-01): endpoint baru `POST /api/v1/auth/admin/login` yang memverifikasi email+password **dan** `role='admin'`, lalu menerbitkan access+refresh token RS256 dengan klaim `role`. Login pengguna reguler (`POST /api/v1/auth/login`) tetap apa adanya (sudah berbasis password).
- **Middleware `require_admin`** (FR-ADM-AUTH-03): middleware otorisasi peran **default-deny** di `common/auth-middleware` — menolak `403 ACCOUNT_NOT_ADMIN` bila klaim `role` ≠ `admin`. Dipasang pada seluruh namespace `/api/v1/admin/**` dan endpoint admin existing (KYC review, suspend).
- **Seed akun admin via DBA** (FR-ADM-AUTH-02): akun admin dibuat lewat migrasi/proses seed (bukan endpoint register). Admin diasumsikan langsung `status = active` (tidak menjalani KYC). **Tidak ada** endpoint self-register admin.
- **Profil admin menyertakan `role`** (FR-ADM-AUTH-04): `GET /api/v1/users/me` mengembalikan `role` agar dashboard menampilkan foto/nama/role.

## Capabilities

### New Capabilities
- `admin-authentication`: login admin email+password tanpa OTP, penerbitan token ber-`role`, dan penyediaan akun admin via seed/DBA (tanpa self-register).
- `role-authorization`: model `role` extensible pada identitas, klaim `role` pada token, dan middleware `require_admin` default-deny yang memproteksi seluruh endpoint `/admin/**`.

### Modified Capabilities
<!-- Tidak ada spec ter-arsip di openspec/specs/. Penambahan role didefinisikan sebagai kapabilitas baru meski memperluas auth-service yang sudah ada. -->

## Impact

- **Kode**: `rust-services/auth-service/` (domain `AuthUser` + `Role`, application service & DTO login admin + named constants `ACCESS_TOKEN_EXPIRY_SECS`/`storage_category`/`DEFAULT_TOS_VERSION`/`DEFAULT_REFRESH_TTL_SECS`/`DEFAULT_ACCESS_TTL_SECS` + shared `issue_tokens` helper, infrastructure `jwt.rs` tambah `role` di `JwtClaims` + repository, interface handler `admin_login` & router), `rust-services/auth-service-client/` (tambah `role` di `AuthClaims` + tipe `Role` `#[non_exhaustive]`), `rust-services/common/auth-middleware/` (middleware `require_admin` default-deny), `rust-services/rejki-app/` (pasang `require_admin` pada router admin lintas service). **Catatan lintas-service**: penambahan field `role` pada `AuthClaims` menyentuh seluruh handler yang memakainya (chat/notif/4 iklan/user) — kompatibel karena hanya menambah field opsional.
- **Basis data** (schema `auth`): kolom `role` pada `auth.users` (default `user`, CHECK extensible) + proses seed admin (migrasi data terpisah / perintah seed idempoten). Tidak ada FK lintas-schema.
- **API** (`/api/v1/`): endpoint baru `POST /auth/admin/login`; seluruh `/api/v1/admin/**` (existing & masa depan) diproteksi `require_admin`.
- **Keamanan & standar**: default-deny least-privilege ([OWASP Authorization Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Authorization_Cheat_Sheet.html)); sesi admin idealnya idle-timeout pendek ([OWASP Session Management](https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html)); envelope `ApiResponse`, error RFC 9457-inspired, propagasi `request_id`.

## Non-Goals

- Tier peran majemuk penuh (super_admin/moderator/support) — sengaja ditunda; skema `role` disiapkan extensible agar penambahannya tidak breaking (PRD Dashboard §2).
- Endpoint moderasi konkret (iklan/pelatihan/report/comms) — dimiliki change masing-masing; proposal ini hanya menyediakan fondasi proteksi.
- MFA / 2FA untuk admin — diusulkan di PRD (§7) sebagai NFR, di luar scope fondasi ini.
- Manajemen akun admin lewat UI (tambah/nonaktifkan admin) — akun admin di-inject DBA pada tahap ini.
- Analytics/role eksekutif (CEO) — memanfaatkan fondasi `role` yang sama kelak, di luar scope.
