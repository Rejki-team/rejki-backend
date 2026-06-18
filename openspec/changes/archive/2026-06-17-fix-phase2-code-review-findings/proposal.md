## Why

Code review pada `feature/dashboard-gap-analysis-backend` menemukan 14 temuan (8 correctness, 6 quality/convention) di auth-service, user-service, common, dan rejki-app. Temuan ini adalah **regresi / gap** dari dua fix changes yang sudah complete (`fix-auth-service-code-review-findings`, `fix-user-service-code-review-findings`).

## What Changes

### P0 — CRITICAL (data integrity & security)
- **C1:** Refresh token expiry tidak ditegakkan — query `DELETE ... WHERE token_hash = $1` tanpa filter `AND expires_at > now()`, nilai expiry di-discard (`_expires_at`). Token expired tetap bisa di-refresh tanpa batas.
- **C2:** OTP brute-force bypass via resend — `save_otp` ON CONFLICT DO UPDATE me-reset `attempts = 0`, memungkinkan siklus 4-salah + 1-resend berulang.

### P1 — HIGH (rate limiting, error contract)
- **C3:** Tidak ada `AppError::TooManyRequests` (429) — `security-baseline.html` sudah mendefinisikan kontrak 429 + Retry-After + `RATE_LIMITED` code.
- **C4:** Rate-limit error → 401/422/500, bukan 429. Butuh `ServiceError` enum + mapping handler.
- **C5:** Redis rate limiter EXPIRE "always" → persistent lockout. Fix: Lua script atomik (INCR + conditional EXPIRE).
- **C6:** CSV CWE-1236 injection bypass — formula check setelah quoting.

### P2 — MEDIUM (CLAUDE.md compliance)
- **C7:** `warn_slow!` absen dari 22 method `PgAuthRepository`.
- **C8:** `async-trait = "0.1"` inline di 5 crate, bukan `{ workspace = true }`.
- **C9:** `suspend_account` 7 positional args + `#[allow(clippy::too_many_arguments)]` tanpa justifikasi.
- **C10:** SuspendedTemp auto-recovery duplikat di `login` dan `admin_login`.

### P3 — LOW (code quality)
- **C11:** `allow()` & `allow_raw()` near-duplicate di rate_limit.rs.
- **C12:** Revoke refresh token SQL copy-paste 4×.

### TEST
- **C13:** 2 integration test gagal — bulk suspend test tidak disesuaikan dengan evidence guard + state machine.

### MISSING
- **C14:** `can_transition_to` tidak punya `SuspendedTemp → SuspendedPermanent`.

## Capabilities

### Modified Capabilities
- `common-errors` — tambah `TooManyRequests` variant
- `auth-rate-limiting` — Lua script atomik, error mapping 429
- `auth-token-integrity` — refresh token expiry enforcement
- `otp-verification` — attempts tidak di-reset oleh resend
- `account-status-lifecycle` — SuspendedTemp → SuspendedPermanent transition
- `user-service-security` — CSV formula injection fix order

## Impact

- **Affected code:** 10 source files, 6 Cargo.toml, 1 integration test file
- **Breaking changes:** `AuthService::login`, `admin_login`, `resend_otp`, `request_change_password_otp` return type berubah dari `anyhow::Error` ke `ServiceError`; `suspend_account` signature berubah dari 7 positional args ke `SuspendAccountParams` struct
- **New deps:** none (Lua script via existing `redis` crate)
- **Tests affected:** `bulk_suspend_test.rs` (2 fixes), auth-service unit tests (mock call site update)

## References
- `CLAUDE.md` §4.1, §4.5, §4.6, §4.7
- `docs/security-baseline.html` (kontrak 429 + OTP brute-force "Parsial")
- `openspec/changes/fix-auth-service-code-review-findings/tasks.md` (tasks 2.1, 4.x, 5.1)
- `openspec/changes/fix-user-service-code-review-findings/tasks.md` (task 3.3)
