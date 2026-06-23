## 1. Role enum — expand auth-service-client

- [x] 1.1 Expand `Role` enum: tambah `SuperAdmin`, `AdminIklan`, `AdminUser`, `Moderator`, `UserVerified`, `User` (existing). Backward compat: `User` tetap default, `#[non_exhaustive]` tetap.
- [x] 1.2 Tambah `fn rank(&self) -> u8` — return rank numerik per variant
- [x] 1.3 Update `fn is_admin(&self)` — return `self.rank() >= 60` (moderator ke atas)
- [x] 1.4 Update `fn as_str(&self)` dan `FromStr` — 6 variant mapping ke snake_case DB value
- [x] 1.5 Hapus `Role::Admin` (kode mati setelah migrasi enum)

## 2. Migration — alter CHECK constraint

- [x] 2.1 Buat `auth-service/migrations/20260619000002_extend_roles.up.sql` — `ALTER TABLE ... DROP CONSTRAINT ... ADD CONSTRAINT ... CHECK (role IN (...))` +UPDATE backfill `'admin'` → `'super_admin'`
- [x] 2.2 Buat `auth-service/migrations/20260619000002_extend_roles.down.sql` — rollback ke CHECK IN ('user', 'admin') + UPDATE `'super_admin'` → `'admin'`
- [x] 2.3 Jalankan migrasi ke dev DB

## 3. AppError baru

- [x] 3.1 Tambah `AppError::InsufficientRole` di `common/errors/src/lib.rs` → `403 FORBIDDEN` + `INSUFFICIENT_ROLE`
- [x] 3.2 Biarkan `AccountNotAdmin` tetap ada (unreachable, backward compat)

## 4. Middleware — require_role factory

- [x] 4.1 Buat `pub fn require_role(min_rank: u8) -> impl Fn(Request, Next) -> Result<Response, AppError>` di `common/auth-middleware/src/lib.rs`
- [x] 4.2 Hapus `pub async fn require_admin(...)` (kode mati)
- [x] 4.3 Export `require_role` dari `lib.rs`

## 5. Router updates — ganti require_admin

- [x] 5.1 `auth-service/src/interface/mod.rs` — `.layer(from_fn(require_admin))` → `.layer(from_fn(require_role(80)))`
- [x] 5.2 `iklan-barang-bekas-service/src/interface/mod.rs` — sama
- [x] 5.3 `iklan-pekerja-service/src/interface/mod.rs` — sama
- [x] 5.4 `iklan-pekerjaan-service/src/interface/mod.rs` — sama
- [x] 5.5 `iklan-pelatihan-service/src/interface/mod.rs` — sama
- [x] 5.6 `user-service/src/interface/mod.rs` — `.layer(from_fn(require_admin))` → `.layer(from_fn(require_role(80)))`
- [x] 5.7 `report-service/src/interface/mod.rs` — `.layer(from_fn(require_admin))` → `.layer(from_fn(require_role(60)))`
- [x] 5.8 `corporate-comms-service/src/interface/mod.rs` — `.layer(from_fn(require_admin))` → `.layer(from_fn(require_role(60)))`

## 6. AuthService guard — admin_login

- [x] 6.1 Update `admin_login()` — ganti `u.role.is_admin()` → `u.role.rank() >= 80`
- [x] 6.2 Update test: ganti `Role::Admin` → `Role::SuperAdmin` di test admin_login

## 7. Test fixtures — update role values

- [x] 7.1 `rejki-app/tests/common/fixtures.rs` — `create_test_admin()` pakai `Role::SuperAdmin`; `create_test_user()` pakai `Role::User`
- [x] 7.2 AuthService unit test — ganti `Role::Admin` → `Role::SuperAdmin` (4 test case)

## 8. §4.9 Mandatory commands

- [x] 8.1 `cargo fmt --all`
- [x] 8.2 `cargo clippy --workspace -- -D warnings`
- [x] 8.3 `cargo check --workspace`
- [x] 8.4 `cargo test --workspace`
- [x] 8.5 `cargo sqlx prepare --workspace`
- [x] 8.6 Jika ada error, fix dan ulangi

## 9. Dokumentasi

- [x] 9.1 Update `docs/implementation-plan-phase-3.html` — W3B-05 ✅ Done, progress 55%→60%
- [x] 9.2 Update `docs/index.html` — tambah ws-multi-tier-rbac row
- [x] 9.3 Update `docs/security-baseline.html` — seksi RBAC: 6 role + hierarchy
- [x] 9.4 Update `docs/brainstorm/phase-3-hardening-analysis.html` — A4 gap card "✅ DONE"
- [x] 9.5 Update `rejki-app/src/openapi.rs` — doc Role enum jadi 6 value

## 10. Swagger & Archive

- [x] 10.1 Update openapi.rs — Role description listing 6 role values
- [x] 10.2 Archive change → `openspec/changes/archive/2026-06-19-ws-multi-tier-rbac/`
- [x] 10.3 Sync main spec → `openspec/specs/rbac/spec.md`
- [x] 10.4 Update memory
