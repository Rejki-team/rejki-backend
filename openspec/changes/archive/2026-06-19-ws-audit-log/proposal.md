## Why

Setiap event autentikasi penting — login, logout, refresh token, percobaan gagal, perubahan password — saat ini hanya dicatat via `tracing::info!` / `tracing::warn!` (log aplikasi). Tidak ada jejak audit persisten yang `IMMUTABLE` (append-only, tidak bisa diedit/dihapus). Ini celah keamanan: tanpa audit log, investigasi insiden (akun diretas, brute-force, penyalahgunaan admin) tidak punya bukti forensik yang bisa diandalkan. Phase 3 hardening mensyaratkan tabel `auth.audit_log` (A3) sebagai fondasi compliance dan sistem deteksi.

## What Changes

- **Migration baru**: `CREATE TABLE auth.audit_log` dengan kolom `id`, `user_id`, `event`, `ip_address`, `user_agent`, `request_id`, `metadata` (JSONB), `created_at`. Append-only — `REVOKE UPDATE, DELETE`.
- **Domain baru**: `AuditEvent` enum + `AuditLogRepository` trait di `auth-service/src/domain/audit_log.rs`.
- **Infrastructure baru**: `PgAuditLogRepository` (INSERT-only) di `auth-service/src/infrastructure/audit_log_repository.rs`.
- **Application**: `AuthService` menerima `Arc<dyn AuditLogRepository>` dan memanggil `log()` di setiap auth event: `login_success`, `login_failed`, `admin_login_success`, `admin_login_failed`, `logout`, `token_refresh`, `otp_sent`, `password_changed`, `password_reset`, `account_suspended`.
- **Interface**: `AuditLogRepository` di-wire ke `AppState` + `router_with_deps_ex`, di-inject ke `AuthService`.
- **Context extraction**: `ip_address` dan `user_agent` diambil dari `axum::extract::ConnectInfo` / header `User-Agent` / `x-request-id` di handler layer, diteruskan sebagai `AuditContext`.
- **Swagger**: Tidak ada endpoint publik baru — ini internal infrastructure. Tidak perlu mirror DTO di `openapi.rs`.

## Capabilities

### New Capabilities

- `audit-log`: Tabel `auth.audit_log` + `PgAuditLogRepository` + `AuditLogRepository` trait + injeksi ke `AuthService` agar setiap auth event tercatat secara immutable (append-only, REVOKE UPDATE/DELETE).

### Modified Capabilities

<!-- Tidak ada spec existing yang berubah requirement-nya -->

## Impact

- **Auth service domain**: file baru `audit_log.rs` (trait + enum)
- **Auth service infrastructure**: file baru `audit_log_repository.rs` (Pg impl)
- **Auth service application**: `AuthService::new()` menerima `Arc<dyn AuditLogRepository>`
- **Auth service interface**: `router_with_deps_ex` + `AppState` + handler wiring
- **Database**: migration baru `20260618000001_create_audit_log.up.sql`
- **Test**: unit test AuditLogRepository trait, integration test INSERT + REVOKE
- **rejki-app**: tambah `audit_log_repo` di wiring main.rs / test common
