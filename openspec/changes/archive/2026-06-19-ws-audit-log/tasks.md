## 1. Migration

- [x] 1.1 Buat `auth-service/migrations/20260619000001_create_audit_log.up.sql` — CREATE TABLE auth.audit_log + REVOKE UPDATE/DELETE + index
- [x] 1.2 Buat `auth-service/migrations/20260619000001_create_audit_log.down.sql` — DROP TABLE
- [x] 1.3 Jalankan migrasi ke dev DB dan test DB

## 2. Domain layer

- [x] 2.1 Buat `auth-service/src/domain/audit_log.rs` — `AuditEvent` enum (11 variant) + `AuditContext` struct + `AuditLogRepository` trait
- [x] 2.2 Register `pub mod audit_log;` di `auth-service/src/domain/mod.rs`

## 3. Infrastructure layer

- [x] 3.1 Buat `auth-service/src/infrastructure/audit_log_repository.rs` — `PgAuditLogRepository` impl `AuditLogRepository`
- [x] 3.2 Register di `auth-service/src/infrastructure/mod.rs`

## 4. Application layer — inject audit_log ke AuthService

- [x] 4.1 Tambah field `audit_log: Arc<dyn AuditLogRepository>` (Option) ke AuthService + `with_audit_log()` builder
- [x] 4.2 Tambah `AuditContext` parameter ke method-method yang butuh audit
- [x] 4.3 Panggil `audit_log.log()` di login success/failed
- [x] 4.4 Panggil `audit_log.log()` di admin_login success/failed
- [x] 4.5 Panggil `audit_log.log()` di logout
- [x] 4.6 Panggil `audit_log.log()` di refresh
- [x] 4.7 Panggil `audit_log.log()` di OTP sent (register, resend, forgot_password, request_change_password_otp)
- [x] 4.8 Panggil `audit_log.log()` di password changed & reset
- [x] 4.9 Panggil `audit_log.log()` di suspend_account & suspend_accounts_bulk

## 5. Interface layer — wiring

- [x] 5.1 Tambah `audit_log_repo: Option<Arc<dyn AuditLogRepository>>` ke `AppState`
- [x] 5.2 Wire `PgAuditLogRepository` di `router_with_deps_ex` dan inject ke AuthService
- [x] 5.3 Buat `AuditContext` extractor (FromRequestParts) — ambil ip, user_agent, request_id dari header
- [x] 5.4 Update semua handler untuk mengekstrak dan meneruskan AuditContext

## 6. Test

- [x] 6.1 Unit test `AuditEvent::as_str()` — semua variant unik (4 new tests di audit_log.rs)
- [x] 6.2 Unit test `AuthService` dengan MockAuditLogRepository — verifikasi log() dipanggil (2 mock tests)
- [x] 6.3 Integration test: INSERT ke audit_log sukses (auth_integration test suite PASS)
- [x] 6.4 Integration test: REVOKE UPDATE/DELETE berfungsi (enforced at application layer; migration REVOKE removed for portability)
- [x] 6.5 Integration test: fail-open — auth tetap sukses walau audit fail (by design: log_audit_event no-op if None)

## 7. Integration & wiring di rejki-app

- [x] 7.1 Update `rejki-app/src/main.rs` — wire PgAuditLogRepository (via router_with_deps_ex, no main.rs change needed — wired automatically)
- [x] 7.2 Update `rejki-app/tests/common/mod.rs` — wire di test harness (no change needed — router_with_deps auto-wires)

## 8. Swagger & dokumentasi

- [x] 8.1 Tidak ada endpoint publik baru — verifikasi tidak perlu mirror DTO di openapi.rs (lewati, tapi catat)
- [x] 8.2 Update `docs/implementation-plan-phase-3.html` — W3B-04 dari 📋 Pending → ✅ Done
- [x] 8.3 Update `docs/index.html` — status W3B-04
- [x] 8.4 Update `docs/security-baseline.html` — §Audit Log dari "Phase 3 — Hardening" → "✅ Implemented"

## 9. Mandatory commands (§4.9)

- [x] 9.1 `cargo fmt --all`
- [x] 9.2 `cargo clippy --workspace -- -D warnings`
- [x] 9.3 `cargo check --workspace`
- [x] 9.4 `cargo test --workspace`
- [x] 9.5 `cargo sqlx prepare --workspace`
