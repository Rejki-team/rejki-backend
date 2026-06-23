## Why

Code review terhadap `auth-service` menemukan **18 temuan** — mencakup **3 race condition** (data integrity), **3 kebocoran anti-enumeration** (security), **1 state machine bypass** (authorization), dan **0% unit test coverage** pada business logic (Acceptance Criteria 85% tidak terpenuhi). Temuan ini harus diperbaiki sebelum production deployment karena berpotensi menyebabkan data inconsistency, account takeover, dan ketidakmampuan audit. Target: semua temuan P0+P1 selesai dalam 1 sprint, P2 dalam sprint berikutnya.

## What Changes

**P0 — Critical (security & correctness):**
- Fix login handler: ganti `AppError::Validation(422)` → `AppError::Unauthorized(401)` + buang detail error internal (anti-enumeration)
- Tambah rate limiting pada `/login` dan `/admin/login` endpoint

**P1 — High (data integrity, race conditions, security):**
- Fix `suspend_one`: validasi `can_transition_to()` sebelum `set_status`, bungkus 3 operasi write dalam DB transaction
- Fix refresh token rotation race: gunakan `DELETE ... RETURNING` + cek `rows_affected`, atau `SELECT FOR UPDATE`
- Fix `bump_otp_attempts` race: bungkus UPDATE + conditional DELETE dalam DB transaction
- Fix Redis rate limiter: gunakan Lua script untuk atomic `INCR` + `EXPIRE`, atau selalu panggil `EXPIRE`
- Fix `admin_login` timing side-channel: jalankan bcrypt verify sebelum role/status check
- Fix password reset/change: revoke token dulu, baru update password (dalam transaction)
- Tambah unit test coverage ke ≥85%: `AuthService`, handlers, `PgAuthRepository`

**P2 — Medium (code quality, standards):**
- Extract `TokenIssuer` trait (DIP) — pisahkan application dari infrastructure `JwtService`
- Extract `RateLimiter` trait (DIP) — pisahkan application dari infrastructure `OtpRateLimiter`
- Extract `row_to_user()` shared helper di `PgAuthRepository`
- Merge `hash_otp` + `hash_token` → `sha256_hex()`
- Hapus `async-trait` dependency (tidak dipakai)
- Sinkronkan `MAX_BULK_SUSPEND_USERS` constant dengan DTO validation
- Extract Redis key prefix ke named constant
- Extract `token_response_json()` helper di handlers
- Konformasi env var naming ke Config Standard (`AUTH_DATABASE_URL` → `DATABASE_URL`, `AUTH_PORT` → `APP_PORT`)

## Capabilities

### New Capabilities
- `auth-rate-limiting`: Rate limiting pada endpoint login (email+password) untuk cegah brute-force — pattern yang sama dengan OTP rate limiter
- `auth-transaction-integrity`: Database transaction wrapping untuk multi-step mutations (suspend, password reset, OTP bump) — atomic rollback pada partial failure
- `auth-timing-safety`: Constant-time authentication path — verifikasi kredensial sebelum authorization check untuk cegah timing side-channel

### Modified Capabilities
- `admin-authentication`: Perbaikan timing side-channel pada `admin_login` — bcrypt verify dijalankan sebelum role/status check. Tidak ada perubahan kontrak API.
- `account-suspension`: Perbaikan `suspend_one` — tambah validasi `can_transition_to()` state machine + transaction wrapping. Tidak ada perubahan kontrak API.
- `password-recovery`: Perbaikan urutan operasi `reset_password`/`change_password` — token revocation terjadi sebelum password update dalam transaction. Tidak ada perubahan kontrak API.
- `otp-verification`: Perbaikan `bump_otp_attempts` race condition — UPDATE + DELETE dibungkus transaction. Redis rate limiter INCR+EXPIRE dibuat atomic via Lua script.
- `account-status-lifecycle`: Perbaikan `suspend_one` — transisi status divalidasi lewat state machine. Transisi tidak sah ditolak.

## Impact

- **Affected code**: `auth-service/src/application/service.rs`, `auth-service/src/infrastructure/pg_repository.rs`, `auth-service/src/infrastructure/rate_limit.rs`, `auth-service/src/interface/handlers.rs`, `auth-service/src/interface/mod.rs`, `auth-service/src/domain/repository.rs`, `auth-service/src/application/dto.rs`, `auth-service/Cargo.toml`, `auth-service/src/main.rs`
- **Affected crates**: `auth-service-client` (trait baru `TokenIssuer`, `RateLimiter` jika diekstrak ke domain layer)
- **Breaking changes**: **Tidak ada** — semua perubahan bersifat internal service, kontrak API eksternal tidak berubah (hanya HTTP status code login dari 422→401 yang merupakan koreksi bug)
- **New dependencies**: Tidak ada (Lua script Redis sudah didukung `redis` crate existing)
- **Tests affected**: Perlu ditambah unit test baru untuk semua method yang difix; existing integration test di `rejki-app/tests/auth_integration_test.rs` harus tetap pass
