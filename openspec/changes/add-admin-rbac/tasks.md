## 1. Migrasi Basis Data (schema `auth`)

- [x] 1.1 Tambah kolom `role TEXT NOT NULL DEFAULT 'user'` + `CHECK (role IN ('user','admin'))` pada `auth.users` (migrasi up + down)
- [x] 1.2 Verifikasi migrasi naik & turun bersih pada DB test; backfill default `user` aman pada baris existing

## 2. Tipe Role & Klaim Token

- [x] 2.1 Tambah tipe `Role { User, Admin }` (`#[non_exhaustive]`) di `auth-service-client`
- [x] 2.2 Tambah field `role` pada `AuthClaims` (aditif; token lama tanpa `role` → `user`)
- [x] 2.3 Tambah field `role` pada `JwtClaims` (`auth-service/src/infrastructure/jwt.rs`); set saat penerbitan token
- [x] 2.4 Pastikan deserialisasi `AuthClaims` di seluruh handler (chat/notif/4 iklan/user) tetap kompatibel — `cargo check --workspace`

## 3. Login Admin (spec: admin-authentication)

- [x] 3.1 DTO `AdminLoginInput { email, password }` + validasi
- [x] 3.2 Application `admin_login`: verifikasi password + `role='admin'` + `status='active'`; anti-enumeration pada gagal
- [x] 3.3 Terbitkan access+refresh token RS256 dengan klaim `role`
- [x] 3.4 Handler & route `POST /api/v1/auth/admin/login`
- [x] 3.5 Integration test: admin valid → token ber-role admin; non-admin/kredensial salah → ditolak tanpa membocorkan akun; user reguler tak bisa lewat endpoint admin

## 4. Otorisasi Peran (spec: role-authorization)

- [x] 4.1 Middleware `require_admin` di `common/auth-middleware`: default-deny, tolak `403 ACCOUNT_NOT_ADMIN` bila `role ≠ admin`
- [x] 4.2 Pasang `require_admin` (di atas `require_auth`) pada router `/api/v1/admin/**`
- [x] 4.3 Lindungi endpoint admin existing: `POST /users/admin/kyc/{id}/review` (user-service) & suspend (auth-service)
- [x] 4.4 Integration test: akses `/admin/**` tanpa token admin → 403; dengan token admin → lolos; tanpa klaim role → 403 (default-deny)

## 5. Seed Akun Admin (spec: admin-authentication)

- [x] 5.1 Proses seed idempoten / migrasi data untuk insert akun admin (DBA inject): `role='admin'`, `status='active'`, `password_hash` di-hash
- [x] 5.2 Pastikan TIDAK ada endpoint self-register admin
- [x] 5.3 Dokumentasikan cara membuat/rotasi akun admin (operasional/DBA)

## 6. Profil Admin

- [x] 6.1 Sertakan `role` pada respons `GET /api/v1/users/me` (untuk header foto/nama/role di dashboard)
- [x] 6.2 Integration test: `GET /me` admin menampilkan `role=admin`

## 7. Wiring & Finalisasi

- [x] 7.1 Wire `require_admin` ke seluruh sub-router admin di `rejki-app`
- [x] 7.2 Pastikan envelope `ApiResponse`, error RFC 9457-inspired, propagasi `request_id`
- [x] 7.3 Pastikan kredensial/role tidak bocor di log
- [x] 7.4 Swagger/OpenAPI update: `AdminLoginDocRequest`, `admin_login_doc()`, `role` field di `UserProfileDocResponse`
- [x] 7.5 `cargo fmt`, `clippy -D warnings`, seluruh test hijau — zero warnings, zero errors

## 8. Code Quality (pasca-implementasi)

- [x] 8.1 Zero Hardcoded: `ACCESS_TOKEN_EXPIRY_SECS`, `DEFAULT_TOS_VERSION`, `DEFAULT_REFRESH_TTL_SECS`, `DEFAULT_ACCESS_TTL_SECS`, `storage_category::SUSPENSION_EVIDENCE` named constants
- [x] 8.2 Zero Duplication: extract `issue_tokens()` shared helper — `login`, `admin_login`, `refresh` kini 1 baris call
- [x] 8.3 Zero `#[allow(clippy::too_many_arguments)]` — verified clean sebelum review
- [x] 8.4 Code review report: `CODE_REVIEW.md` — 30 poin compliance, 4 temuan (semua fixed)
