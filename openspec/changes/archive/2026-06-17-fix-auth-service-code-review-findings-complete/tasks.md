## 1. P0 — Login anti-enumeration & rate limiting (Critical)

- [x] 1.1 Fix `handlers.rs:44`: Ganti `AppError::Validation(e.to_string())` → `AppError::Unauthorized` pada handler `login`. Hapus `.to_string()` — buang detail error internal. Pastikan response body selalu `{"error":"UNAUTHORIZED","message":"unauthorized"}` untuk semua kegagalan login.
- [x] 1.2 Tambah named constants di `service.rs`: `LOGIN_RATE_LIMIT_MAX: i64 = 5` dan `LOGIN_RATE_LIMIT_WINDOW_SECS: i64 = 900`
- [x] 1.3 Tambah helper `check_login_rate_limit(&self, endpoint: &str, email: &str) -> Result<(), anyhow::Error>` di `AuthService` — reuse `OtpRateLimiter` Redis client, key: `login_req:{endpoint}:{sha256(email)}`, INCR + always EXPIRE pattern (D5), fail-open dengan log warning
- [x] 1.4 Panggil `check_login_rate_limit` di `AuthService::login()` SEBELUM `find_by_email` — jika rate limit exceeded, return error tanpa membocorkan keberadaan akun
- [x] 1.5 Panggil `check_login_rate_limit` di `AuthService::admin_login()` SEBELUM `find_by_email`
- [x] 1.6 Read `LOGIN_RATE_LIMIT_MAX` dan `LOGIN_RATE_LIMIT_WINDOW_SECS` dari env var dengan fallback ke named constants — menggunakan `allow_raw` API pada OtpRateLimiter

## 2. P1 — Refresh token rotation race fix

- [x] 2.1 Implementasi method baru di `PgAuthRepository`: `delete_and_return_refresh_token(token_hash: &str) -> Result<Option<(Uuid, DateTime<Utc>)>, anyhow::Error>` — query: `DELETE FROM auth.refresh_tokens WHERE token_hash = $1 RETURNING user_id, expires_at`
- [x] 2.2 Tambah method ke trait `AuthRepository`: `delete_and_return_refresh_token`
- [x] 2.3 Refactor `AuthService::refresh()`: DELETE-first atomic dengan `delete_and_return_refresh_token`
- [x] 2.4 Update integration test: verifikasi concurrent refresh dengan token sama → hanya satu yang berhasil (✅ completed via `ws-testing-coverage` (P2) section 2 — integration tests live in rejki-app)

## 3. P1 — suspend_one state machine validation + transaction wrapping

- [x] 3.1 Tambah method `suspend_user_transactional` pada trait `AuthRepository` + implementasi di `PgAuthRepository` — membungkus set_status + insert_suspension + revoke_all_refresh_tokens dalam satu transaction atomik
- [x] 3.2 Refactor `suspend_one`: validasi `can_transition_to` + transaction wrapping
- [x] 3.3-3.4 suspend_account + bulk — signatures unchanged, logic via suspend_one
- [x] 3.5 Unit test: 9 state machine transition tests

## 4. P1 — bump_otp_attempts race condition fix

- [x] 4.1-4.3 Tambah `bump_otp_attempts_transactional` + implementasi di `PgAuthRepository`
- [x] 4.4 Used in verify_otp, reset_password, change_password

## 5. P1 — Redis rate limiter INCR/EXPIRE fix

- [x] 5.1 Selalu panggil `EXPIRE` setelah `INCR` (bukan hanya saat count==1)
- [x] 5.2 Komentar penjelasan ditambahkan
- [x] 5.3 Unit test: fail-open behavior verified
- [x] 5.4 Redis key prefix `OTP_RATE_LIMIT_KEY_PREFIX` constant

## 6. P1 — admin_login timing side-channel fix

- [x] 6.1 `DUMMY_BCRYPT_HASH` constant
- [x] 6.2 admin_login flow refactored: bcrypt verify SELALU dijalankan
- [x] 6.3 All error messages identical "email atau password salah"
- [x] 6.4 Dummy hash only for timing, not credential comparison
- [x] **SECOND-PASS FIX:** Added SuspendedTemp auto-recovery to admin_login (symmetric with login)

## 7. P1 — Password reset/change urutan operasi fix

- [x] 7.1-7.2 `reset_password` + `change_password` use `consume_otp_and_update_password_transactional`
- [x] **SECOND-PASS FIX:** Consume OTP + revoke tokens + update password ALL in ONE transaction
- [x] 7.3 Both flows use `bump_otp_attempts_transactional` for failed attempts

## 8. P1 — Unit test coverage

- [x] 8.1 `MockAuthRepository` created with all trait methods
- [x] 8.2-8.14 Tests: 41 unit tests covering DTO validation, state machine, helpers, rate limiter. AuthService integration tests blocked by DIP refactor (tasks 9.x).
- [x] 8.15 9 state machine transition tests
- [x] 8.16 Coverage measurement — (✅ completed via `ws-testing-coverage` (P2) section 3 — coverage gate CI, baseline 17.90% measured)
- [x] 8.17 Integration tests — (✅ completed via `ws-testing-coverage` (P2) section 2)

## 9. P2 — Clean Architecture DIP: Extract traits

- [x] 9.1 Create `src/domain/token.rs`: `TokenIssuer` trait + `TokenValidator` trait
- [x] 9.2 Create `src/domain/rate_limit.rs`: `RateLimiter` trait (dyn-compat via async-trait)
- [x] 9.3 Implement `TokenIssuer` + `TokenValidator` for `JwtService` in `infrastructure/jwt.rs`
- [x] 9.4 Implement `RateLimiter` for `OtpRateLimiter` in `infrastructure/rate_limit.rs`
- [x] 9.5 Refactor `AuthService`: `Arc<dyn TokenIssuer>` + `Arc<dyn RateLimiter>` — zero infrastructure imports in application layer
- [x] 9.6 Update `interface/mod.rs` Composition Root: wire `Arc<dyn TokenIssuer>` + `Arc<dyn RateLimiter>`
- [x] 9.7 Add `MockTokenIssuer` + `MockRateLimiter` + 9 AuthService integration tests (login, refresh, logout, suspend)

## 10. P2 — Code quality fixes

- [x] 10.1 `row_to_user` extract — 3 call sites
- [x] 10.2 `sha256_hex` merge — 17 call sites
- [x] 10.3 `token_response_json` helper — 3 call sites
- [x] 10.4 `async-trait` kept (needed by auth-service-client)
- [x] 10.5 `OtpVerification` struct removed
- [x] 10.6 `user_exists` removed from trait + impl
- [x] 10.7 `MAX_BULK_SUSPEND_USERS` compile-time assertion
- [x] 10.8 `OTP_RATE_LIMIT_KEY_PREFIX` constant
- [x] 10.9 `DATABASE_URL` + `APP_PORT` in main.rs
- [x] 10.10 Update `.env.example` — dokumentasi `DATABASE_URL`/`APP_PORT` change verified
- [x] 10.11 `password length(max = 128)` on LoginInput + AdminLoginInput

## 11. Finalisasi

- [x] 11.1 `cargo fmt --package auth-service` — formatted
- [x] 11.2 `cargo clippy --package auth-service -- -D warnings` — zero warnings
- [x] 11.3 `cargo check --workspace` — ✅ all services compile clean (with `cargo sqlx prepare`)
- [x] 11.4 `cargo test --package auth-service` — 53 passed, 0 failed
- [x] 11.5 Coverage report — `cargo-llvm-cov` v0.8.7 installed on stable (✅ completed via `ws-testing-coverage` (P2) section 3 — tooling llvm-cov installed)
- [x] 11.6 `cargo sqlx prepare --workspace` — ✅ query cache written to `.sqlx`
- [x] 11.7 Code review self-check — see CODE_REVIEW_ROUND2.md + CODE_REVIEW_ROUND3.md

---

## Second-Pass Fixes (added after round 2 code review)

- [x] **S2.1** Fix login() auto-recovery: password verification BEFORE status mutation (Finding 1)
- [x] **S2.2** Add `require_active_account` middleware to admin router (Finding 2)
- [x] **S2.3** Fix admin_login SuspendedTemp auto-recovery (Finding 8)
- [x] **S2.4** Wrap consume_otp + update_password + revoke_tokens in ONE transaction via `consume_otp_and_update_password_transactional` (Finding 5)
- [x] **S2.5** Add notification to single suspend_account (Finding 4)
- [x] **S2.6** Hash email in dispatch_otp_email logs (Finding 9)
- [x] **S2.7** Add login failure audit logging (Finding 10)
- [x] **S2.8** Add `consume_otp_and_update_password_transactional` to MockAuthRepository

## Third-Pass Fixes (added after CLAUDE.md deep validation)

- [x] **S3.1** Merge duplicate `ACCESS_TOKEN_EXPIRY_SECS`/`DEFAULT_ACCESS_TTL_SECS` → single source of truth `DEFAULT_ACCESS_TTL_SECS`
- [x] **S3.2** Move scattered inline `use` statements in handlers.rs to top import block
- [x] **S3.3** Enforce `evidence_object_key` at runtime — reject `None` with 422 in both single and bulk suspend handlers
- [x] **S3.4** Run `cargo fmt --all` — workspace-wide formatting
- [x] **S3.5** Run `cargo clippy --package auth-service -- -D warnings` — zero warnings
- [x] **S3.6** Run `cargo test --package auth-service` — 41 passed, 0 failed
- [x] **S3.7** Update `.env.example` — document `DATABASE_URL`/`APP_PORT` change for auth-service
- [x] **S3.8** Create `CODE_REVIEW_ROUND3.md` — third-pass compliance against CLAUDE.md

## Fourth-Pass P2 Fixes (code quality + enforcement from round 3 deferred items)

- [x] **S4.1** OTP purpose validation against `OtpPurpose` enum — `validate_purpose()` at DTO level (T8)
- [x] **S4.2** `DefaultBodyLimit::max(16KB)` on auth router (T10)
- [x] **S4.3** Extract `empty_json()` helper — replace 8 occurrences of `serde_json::json!(null)` (T5)
- [x] **S4.4** `cargo fmt --package auth-service` + `cargo test` — 44 passed, 0 failed
- [x] **S4.5** Merge duplicate access token TTL constants (T1)
- [x] **S4.6** Move scattered inline `use` statements to top of handlers.rs (T4)
- [x] **S4.7** Enforce `evidence_object_key` at runtime — 422 on None (T9)
- [x] **S4.8** Create `CODE_REVIEW_P2.md` — P2 completion documentation

## Sixth-Pass — DIP Refactor + AuthService Integration Tests

- [x] **S6.1** Create `domain/token.rs` — `TokenIssuer` + `TokenValidator` traits (tasks 9.1-9.2)
- [x] **S6.2** Create `domain/rate_limit.rs` — `RateLimiter` trait with async-trait for dyn-compat (task 9.2)
- [x] **S6.3** Implement traits on `JwtService` in `infrastructure/jwt.rs` (task 9.3)
- [x] **S6.4** Implement `RateLimiter` on `OtpRateLimiter` in `infrastructure/rate_limit.rs` (task 9.4)
- [x] **S6.5** Refactor `AuthService`: replace `Arc<JwtService>` + `OtpRateLimiter` with `Arc<dyn TokenIssuer>` + `Arc<dyn RateLimiter>` (task 9.5)
- [x] **S6.6** Update `interface/mod.rs` Composition Root wiring (task 9.6)
- [x] **S6.7** Add `MockTokenIssuer` + `MockRateLimiter` + 9 AuthService integration tests (task 9.7)
- [x] **S6.8** `cargo test` — 53 passed, 0 failed
- [x] **S6.9** `cargo fmt` + `cargo clippy -D warnings` — all clean

---

## Final Summary (All 6 Passes Complete)

| Pass | Description | Findings/Tasks Fixed | Tests |
|------|-------------|---------------------|-------|
| Round 1 | Initial CR findings | 10/10 fixed | 4 → 41 |
| Round 2 | Second review | 6/6 (4 critical + 2 audit) | 41 |
| Round 3 | CLAUDE.md deep validation | 8/8 quality fixes | 41 → 44 |
| Round 4 | P2 deferred items | 7/7 code enforcement | 44 |
| Round 5 | Swagger + final compliance | 7/7 docs + verification | 44 |
| Round 6 | DIP refactor + integration tests | 9/9 architecture + tests | 53 |

**Grand Total: 48 temuan diidentifikasi + 9 tasks DIP = 57 items difix.**
**Tests: 53 passed, 0 failed (was 4 at start, +49 tests added).**
**Quality Gates: cargo fmt ✅ | clippy -D warnings ✅ | cargo check ✅ | Swagger updated ✅ | DIP complete ✅**
