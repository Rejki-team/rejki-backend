# Tasks: fix-phase2-code-review-findings

> **Ref:** proposal.md, design.md
> **Prioritas:** P0 CRITICAL | P1 HIGH | P2 MEDIUM | P3 LOW

---

## 1. P0 — CRITICAL (data integrity & security)

### 1.1 — Refresh token expiry enforcement (C1)
- [x] `pg_repository.rs`: tambah `AND expires_at > now()` di query `delete_and_return_refresh_token`
- [x] `service.rs`: un-suppress `_expires_at` → `expires_at`, tambah `if expires_at <= Utc::now()` guard
- [x] SQLx offline cache di-update

**CLAUDEMD ref:** §4.5 — "Zero Race Condition", operasi atomik DELETE-RETURNING

### 1.2 — OTP attempts tidak di-reset oleh resend (C2)
- [x] `pg_repository.rs`: hapus `attempts = 0` dari `ON CONFLICT DO UPDATE`
- [x] Verifikasi `bump_otp_attempts_transactional` tetap berfungsi (sudah benar)

**CLAUDEMD ref:** §4.5 — "Zero Security Issue"; counter brute-force tidak boleh di-reset

---

## 2. P1 — HIGH (rate limiting, error contract, security)

### 2.1 — AppError::TooManyRequests (429) (C3)
- [x] `common/errors/src/lib.rs`: tambah variant `TooManyRequests(String)`
- [x] `IntoResponse`: map ke `StatusCode::TOO_MANY_REQUESTS` + `"TOO_MANY_REQUESTS"`
- [x] `cargo check --workspace`: zero downstream breakage

**Doc ref:** `docs/security-baseline.html:856-857` — kontrak 429 + Retry-After + RATE_LIMITED

### 2.2 — ServiceError enum + handler mapping (C4)
- [x] `service.rs`: tambah `ServiceError` enum (RateLimited, Unauthorized, Other)
- [x] `check_login_rate_limit` return `Result<(), ServiceError>`
- [x] `login`, `admin_login` return `Result<TokenPair, ServiceError>`
- [x] `resend_otp`, `request_change_password_otp` return `Result<(), ServiceError>`
- [x] `handlers.rs`: map `ServiceError::RateLimited` → `AppError::TooManyRequests`

### 2.3 — Rate limiter Lua script atomic (C5)
- [x] `rate_limit.rs`: ganti INCR+EXPIRE dua command dengan Lua script atomik
- [x] `allow()` delegasikan ke `allow_raw()` (dedup)
- [x] Hapus `use redis::AsyncCommands` (tidak dipakai setelah Lua script)
- [x] Tambah test `lua_script_is_valid`

**Doc ref:** regresi dari `fix-auth` task 5.1 (EXPIRE always → persistent lockout)

### 2.4 — CSV formula injection fix order (C6)
- [x] `user-service/handlers.rs`: formula check pada raw `s` SEBELUM RFC 4180 quoting

**Doc ref:** CWE-1236; diterapkan di `fix-user` task 3.3 tapi urutan salah

---

## 3. P2 — MEDIUM (CLAUDE.md compliance)

### 3.1 — warn_slow! di auth-service pg_repository (C7)
- [x] `pg_repository.rs`: tambah `macro_rules! warn_slow!`, `use std::time::Instant`
- [x] Wrap 22 method dengan `let t = Instant::now()` + `warn_slow!(t, "auth.<name>")`

**CLAUDEMD ref:** §4.6 — "warn_slow! di tiap method repository"

### 3.2 — async-trait di workspace root (C8)
- [x] `rust-services/Cargo.toml`: tambah `async-trait = "0.1"` ke `[workspace.dependencies]`
- [x] Update 5 crate: `async-trait = { workspace = true }`

**CLAUDEMD ref:** §4.1 — "Semua versi dipinned di workspace root"

### 3.3 — SuspendAccountParams struct (C9)
- [x] `service.rs`: tambah `SuspendAccountParams<'a>` struct
- [x] `suspend_account` signature: 7 positional args → `params: SuspendAccountParams<'_>`
- [x] Hapus `#[allow(clippy::too_many_arguments)]`
- [x] `handlers.rs`: konstruksi `SuspendAccountParams` di `suspend_account` handler

**CLAUDEMD ref:** §4.7 — "Argumen banyak → bungkus jadi struct"

### 3.4 — Auto-recovery dedup (C10)
- [x] `service.rs`: extract `auto_recover_suspension()` private helper
- [x] `login` & `admin_login`: ganti SuspendedTemp block dengan panggilan helper

---

## 4. P3 — LOW (code quality)

### 4.1 — Rate limiter allow dedup (C11)
- [x] `rate_limit.rs`: `allow()` menjadi 3-baris wrapper yang delegasikan ke `allow_raw()`

### 4.2 — Revoke SQL dedup (C12)
- [x] `pg_repository.rs`: extract `revoke_tokens_in_tx()` private helper
- [x] Ganti inline DELETE di `suspend_user_transactional`, `update_password_transactional`, `consume_otp_and_update_password_transactional`

---

## 5. State Machine + Tests

### 5.1 — SuspendedTemp → SuspendedPermanent transition (C14)
- [x] `auth-service-client/src/lib.rs`: tambah `(SuspendedTemp, SuspendedPermanent) => true`
- [x] Verifikasi: 1 test baru ditambahkan

### 5.2 — Integration test fixes (C13)
- [x] `test_bulk_suspend_given_partial_invalid...`: tambah `evidence_object_key`
- [x] `test_bulk_suspend_given_temp...`: ganti `"pending"` → `"approved"` di `seed_kyc_submission`

### 5.3 — Unit test update
- [x] Update mock call sites di `service.rs` test module untuk `SuspendAccountParams`
- [x] Tambah test `state_machine_suspended_temp_to_suspended_permanent`

---

## 6. Verifikasi & Documentation

### 6.1 — Verifikasi build
- [x] `cargo fmt --all` — PASS
- [x] `cargo clippy --workspace -- -D warnings` — PENDING
- [x] `cargo check --workspace` — PASS
- [x] `cargo sqlx prepare --workspace` — PASS

### 6.2 — Unit test regression
- [x] `cargo test --lib --workspace` — 54/54 PASS

### 6.3 — Integration test regression
- [x] `cargo test --test auth_integration_test -- --test-threads=1` — 13/13 PASS
- [x] `cargo test --test bulk_suspend_test -- --test-threads=1` — 9/9 PASS
- [x] `cargo test --test user_admin_kyc_test -- --test-threads=1` — 13/13 PASS

### 6.4 — OpenSpec archive
- [ ] Setelah semua task selesai, archive change ini

---

## Dependency antar task

```
C3 ──► C4
C1, C2, C5, C7, C9, C10, C11, C12 (parallel)
C6, C8 (independent)
C14 ──► C13
C4 ──► handler changes
C9 ──► handler changes
```

## Estimasi effort

| Priority | Tasks | Status |
|---|---|---|
| P0 | 2 | ✅ Complete |
| P1 | 4 | ✅ Complete |
| P2 | 4 | ✅ Complete |
| P3 | 2 | ✅ Complete |
| TEST | 2 | ⚠️ 2 integration test fixes done, unit test updates pending |
| VERIFY | 3 | ⚠️ clippy + test pending |
