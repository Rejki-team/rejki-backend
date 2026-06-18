## Context

Rejki adalah modular monolith Rust; auth-service memiliki state machine `AccountStatus` (`auth-service-client/src/lib.rs`) tetapi **tidak ada konsep `role`**. Login pengguna reguler sudah berbasis **password** (`LoginInput { email, password }`, `auth-service/src/application/dto.rs`), bukan OTP. JWT RS256 di-wire via `JwtService` di `rejki-app`. Endpoint admin yang sudah ada (`POST /api/v1/users/admin/kyc/{id}/review` di user-service; suspend di auth-service) menandai di komentar bahwa "proteksi RBAC menyusul di proposal Web Dashboard". Proposal ini adalah proposal tersebut: fondasi RBAC bagi seluruh change moderasi dashboard. Acuan: User Story Dashboard (FR-ADM-AUTH-01..05) & keputusan pemilik produk (satu role `admin` extensible; login admin email+password tanpa OTP; akun di-inject DBA).

## Goals / Non-Goals

**Goals:**
- Menambah `role` extensible pada identitas + klaim token, tanpa merusak handler existing.
- Endpoint login admin email+password (tanpa OTP) yang memverifikasi `role=admin`.
- Middleware `require_admin` default-deny memproteksi seluruh `/admin/**`.
- Menyediakan akun admin via seed/DBA (tanpa self-register).

**Non-Goals:**
- Tier peran majemuk penuh, MFA admin, manajemen admin via UI, role eksekutif CEO.

## Decisions

### D1 — `role` sebagai kolom enum extensible, terpisah dari `status`
`role` adalah dimensi **orthogonal** terhadap `status` akun. Tambah `role TEXT NOT NULL DEFAULT 'user'` dengan `CHECK (role IN ('user','admin'))`. Memakai `TEXT + CHECK` (bukan enum Postgres native) agar penambahan nilai di masa depan cukup mengubah `CHECK` (migrasi aditif) tanpa `ALTER TYPE` yang lebih rumit — konsisten dengan pendekatan `status` yang juga `TEXT + CHECK`. Tipe domain Rust `Role { User, Admin }` (`#[non_exhaustive]` agar penambahan varian tidak breaking di sisi konsumen).

### D2 — `role` ditambahkan ke `JwtClaims` & `AuthClaims` secara aditif
Tambah `role: Role` (atau `Option<Role>` untuk backward-compat token lama) pada `JwtClaims` (`auth-service/src/infrastructure/jwt.rs`) dan `AuthClaims` (`auth-service-client`). Karena hanya menambah field, seluruh handler yang mendeserialisasi `AuthClaims` tetap kompatibel. Token lama tanpa `role` diperlakukan sebagai `user` (default deny untuk admin). Embedding `role` di JWT menjaga kecepatan otorisasi tanpa query DB per-request (selaras pola `status` di `extend-auth-service-onboarding`).

### D3 — Endpoint login admin TERPISAH (`POST /auth/admin/login`)
Memisahkan dari `/auth/login` reguler memungkinkan: (a) verifikasi tambahan `role=admin` dengan galat seragam `403`/`401` anti-enumeration; (b) kebijakan sesi admin lebih ketat (idle timeout pendek 2–5 menit, [OWASP Session Management](https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html)) tanpa menyentuh alur mobile; (c) audit login admin terpisah. Alur: validasi kredensial → cek `role=admin` & `status=active` → terbitkan access+refresh token dengan klaim `role`. Non-admin yang memakai endpoint ini ditolak tanpa membocorkan keberadaan akun.

### D4 — Middleware `require_admin` default-deny
Middleware baru di `common/auth-middleware` membaca `AuthClaims` dari extensions (di-set `require_auth`) dan menolak `403` kode mesin `ACCOUNT_NOT_ADMIN` bila `role ≠ admin`. **Default-deny**: tanpa klaim `role` yang valid → ditolak ([OWASP Authorization Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Authorization_Cheat_Sheet.html)). Dipasang sebagai layer di atas `require_auth` pada sub-router `/admin/**`. Menjadi titik penegakan tunggal yang dipakai ulang oleh change moderasi lain.

### D5 — Akun admin via seed/DBA, tanpa self-register
Tidak ada endpoint register admin. Akun admin dibuat lewat proses seed idempoten / migrasi data (DBA inject): insert `auth.users` dengan `role='admin'`, `status='active'`, `password_hash` di-hash (Argon2/sesuai util existing). Kredensial awal dikelola operasional (di luar kode). Admin **tidak** menjalani state machine KYC. Alternatif "promosikan user jadi admin via endpoint" ditolak pada tahap ini (butuh manajemen admin/UI — Non-Goal).

### D6 — Kepatuhan standar Phase 1.5
Envelope `ApiResponse`, error RFC 9457-inspired, prefix `/api/v1/`, propagasi `request_id`, snake_case DB, `created_at`/`updated_at`. Login admin mengikuti pola anti-enumeration yang sudah ada di auth.

### D7 — Zero Hardcoded: Named Constants
Semua nilai numerik dan string yang sebelumnya hardcoded diganti dengan named constants:
- `ACCESS_TOKEN_EXPIRY_SECS: u64 = 900` — durasi access token
- `DEFAULT_TOS_VERSION: &str = "v1"` — versi T&C default
- `DEFAULT_REFRESH_TTL_SECS: i64 = 2_592_000` — fallback refresh TTL
- `DEFAULT_ACCESS_TTL_SECS: i64 = 900` — fallback access TTL
- `storage_category::SUSPENSION_EVIDENCE: &str = "suspension-evidence"` — kategori storage

### D8 — Shared `issue_tokens` Helper
Fungsi `login`, `admin_login`, dan `refresh` memiliki logika identik untuk menerbitkan JWT + menyimpan refresh token (~15 baris). Logic ini diekstrak ke `issue_tokens(&self, user, effective_status) -> Result<TokenPair>` untuk menghindari duplikasi. Ketiga call site kini 1 baris.

## Risks / Trade-offs

- **Field baru `role` menyentuh banyak handler (via `AuthClaims`)** → mitigasi: aditif & `Option`/default `user`; build lintas-service tetap kompatibel. Diuji dengan `cargo check --workspace`.
- **Kredensial admin awal** → di-seed oleh DBA; risiko kredensial lemah dikelola operasional (rotasi, kebijakan password yang sudah diperketat di `extend-auth`).
- **Penyalahgunaan endpoint admin** → default-deny + audit; idealnya sesi admin idle-timeout pendek + MFA (NFR PRD, di luar scope kode ini).
- **Migrasi `role` pada tabel existing** → `DEFAULT 'user'` membuat backfill aman; rollback drop kolom (nullable saat transisi).

## Migration Plan

1. Migrasi `auth.users`: tambah `role TEXT NOT NULL DEFAULT 'user'` + `CHECK (role IN ('user','admin'))` (up + down).
2. Tambah tipe `Role` di `auth-service-client`; tambah `role` di `AuthClaims` & `JwtClaims` (aditif).
3. Implementasi `admin_login` (verifikasi password + `role=admin` + `status=active`) + handler/route `POST /auth/admin/login`.
4. Implementasi middleware `require_admin` di `common/auth-middleware`.
5. Pasang `require_admin` pada router `/admin/**` lintas service (KYC review, suspend, dan endpoint admin masa depan).
6. Proses seed admin idempoten (DBA inject) + dokumentasi cara membuat akun admin.
7. Sertakan `role` pada `GET /users/me`.
8. Rollback: migrasi turun drop kolom `role`; klaim `role` opsional sehingga token lama tetap valid; endpoint admin tanpa middleware kembali ber-placeholder.

## Resolved Questions

Diputuskan oleh pemilik produk (2026-06-14, via PRD Dashboard v0.2):
- **Satu role `admin`** dulu, skema extensible ke tier (super_admin/moderator/support) tanpa breaking change.
- **Login admin email+password tanpa OTP**; akun di-inject DBA; tanpa self-register.
- Admin diasumsikan langsung `active` (tidak menjalani KYC).

## Open Questions

- **Manajemen akun admin** (tambah/nonaktifkan, ganti password admin) lewat UI — ditunda hingga ada kebutuhan; saat ini operasional/DBA.
- **MFA admin** — diusulkan sebagai NFR; mekanisme (TOTP/email OTP khusus admin) belum ditetapkan.
- **Idle/absolute timeout sesi admin** — nilai pasti (mis. 5 menit idle) perlu konfirmasi kebijakan keamanan sebelum produksi.
