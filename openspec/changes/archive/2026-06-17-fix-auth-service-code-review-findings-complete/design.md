## Context

Code review terhadap `auth-service` (16 file, ~1500 LOC) menemukan 10 confirmed findings: 3 race conditions, 3 security bypass, 1 state machine bypass, 1 HTTP status mismatch, 1 missing rate limit, dan 0% unit test coverage pada business logic. Semua temuan berada dalam satu service (`auth-service`) sehingga tidak ada cross-service migration. Service sudah memiliki `OtpRateLimiter` (Redis-based) dan `PgPool` — tidak ada dependency baru.

Dependensi lintas-service: `auth-service-client` (shared types), `common-errors` (AppError), `common-auth-mw` (require_auth, require_admin), `notification-service-client`, `storage-service-client`, `user-service-client`.

## Goals / Non-Goals

**Goals:**
- Menutup semua race condition (refresh token rotation, OTP bump, Redis INCR/EXPIRE)
- Menutup semua bypass anti-enumeration (login 422 → 401, admin_login timing side-channel)
- Menutup state machine bypass di suspend_one
- Memperbaiki data integrity (transaction wrapping untuk multi-step write)
- Menambah rate limiting pada endpoint login
- Mencapai ≥85% unit test coverage
- Extract trait untuk DIP (Clean Architecture)
- Menghilangkan duplikasi kode dan dead code
- Konformasi env var naming ke Config Standard Phase 1.5

**Non-Goals:**
- Tidak mengubah kontrak API eksternal (kecuali koreksi HTTP status login: 422→401)
- Tidak menambah dependensi eksternal baru
- Tidak mengubah struktur tabel database
- Tidak menyentuh service lain

## Decisions

### D1 — Login rate limiting: reuse `OtpRateLimiter` pattern dengan Redis key baru

`POST /login` dan `POST /admin/login` akan menggunakan Redis counter `login_req:{email}` dengan window 15 menit, max 5 percobaan gagal (lebih generous dari OTP karena login harus lebih toleran terhadap user error). Rate limit dicek di application service (`AuthService::login` dan `admin_login`) SEBELUM bcrypt verify — jika rate limit exceeded, return error tanpa membocorkan apakah email terdaftar.

**Alternatives considered:**
- Middleware rate limit per IP — ditolak karena tidak efektif untuk distributed brute-force (attacker bisa rotating IP)
- Check setelah password verify — ditolak karena CPU waste (bcrypt mahal)

### D2 — Refresh token rotation: DELETE-first dengan RETURNING + error handling

Flow baru `refresh()`:
1. `DELETE FROM auth.refresh_tokens WHERE token_hash = $1 RETURNING user_id, expires_at` — atomic: token dihapus DAN user_id didapat dalam satu operasi
2. Jika `rows_affected = 0` → return error ("refresh token tidak valid atau sudah expired")
3. `find_by_id(user_id)` — ambil data user
4. `issue_tokens(user)` — terbitkan token baru

Dengan DELETE-first, concurrent request kedua akan gagal di step 2 (token sudah tidak ada). Tidak ada race window.

**Alternatives considered:**
- `SELECT ... FOR UPDATE` dalam transaction — ditolak karena lebih kompleks (butuh explicit BEGIN/COMMIT) dan lebih lambat (row lock contention)
- `DELETE` + cek `rows_affected` tanpa RETURNING — ditolak karena butuh query kedua untuk user_id

### D3 — `suspend_one` fix: transaction + state machine validation

Flow baru `suspend_one`:
1. Cek user exists + current status via `find_by_id`
2. Validasi `current_status.can_transition_to(target_status)` — jika false, return error
3. `BEGIN TRANSACTION`
4. `set_status(user_id, target)`
5. `insert_suspension(...)`
6. `revoke_all_refresh_tokens(user_id)`
7. `COMMIT` — jika step 4-6 ada yang gagal, ROLLBACK otomatis via `?`

Semua dalam satu `sqlx::Transaction` untuk atomic rollback.

### D4 — `bump_otp_attempts` fix: transaction wrapping

Flow baru `bump_otp_attempts`:
1. `BEGIN TRANSACTION`
2. `UPDATE ... SET attempts = attempts + 1 ... RETURNING attempts`
3. Jika attempts >= max: `DELETE FROM ... WHERE user_id AND purpose`
4. `COMMIT`

Dengan transaction, concurrent `save_otp` (yang upserts) tidak bisa interleave antara UPDATE dan DELETE.

**Note:** sqlx `query!` macro tidak mendukung `BEGIN`/`COMMIT` sebagai raw SQL dalam macro — gunakan `sqlx::query()` atau `.begin()` pada pool/connection.

### D5 — Redis rate limiter fix: selalu panggil EXPIRE

Daripada menambah Lua script (yang memerlukan `redis::Script`), gunakan pendekatan sederhana: SELALU panggil `EXPIRE` setelah `INCR`, bukan hanya saat `count == 1`. `EXPIRE` pada key yang sudah memiliki TTL hanya memperbarui TTL — tidak mereset counter. Ini menghilangkan race condition dengan biaya 1 extra Redis command per request (bisa di-pipeline).

```rust
let count: i64 = conn.incr(key, 1).await?;
// Selalu perbarui EXPIRE — safe, memperpanjang window untuk request aktif
let _: bool = conn.expire(key, WINDOW_SECS).await?;
Ok(count <= MAX_REQUESTS)
```

**Trade-off:** Setiap request sukses dalam window akan me-reset TTL ke WINDOW_SECS penuh. Artinya rate limit window menjadi "sliding" dari request terakhir, bukan fixed-window murni. Ini justru lebih baik untuk keamanan (sliding window lebih sulit di-bypass).

**Alternatives considered:**
- Lua script — lebih atomic tapi menambah kompleksitas, testability lebih rendah
- Redis MULTI/EXEC — tidak mendukung conditional logic

### D6 — `admin_login` timing fix: selalu jalankan bcrypt verify

Flow baru `admin_login`:
1. `find_by_email(email)` — dapatkan user (Some/None)
2. Jalankan `verify(password, hash)` TERLEBIH DAHULU — gunakan dummy hash untuk user yang tidak ditemukan atau non-admin. Ini membuat timing konstan (~250ms untuk semua case).
3. Cek role + status setelah verify — return error dengan pesan seragam jika gagal

**Implementation:** Gunakan dummy bcrypt hash konstan (`$2b$12$...` dari seed migration) untuk dummy path. Semua path menjalankan bcrypt verify — menghilangkan timing side-channel sepenuhnya.

**Alternatives considered:**
- `verify_password` async spawned di background thread untuk semua path — over-engineering
- Delay timer (sleep) untuk menyamakan timing — tidak akurat di bawah GC pause

### D7 — Password reset/change fix: revoke sebelum update

Flow baru `reset_password`:
1. Verifikasi OTP (consume_otp + bump_otp_attempts)
2. Hash password baru
3. `BEGIN TRANSACTION`
4. `revoke_all_refresh_tokens(user_id)`
5. `update_password(user_id, new_hash)`
6. `COMMIT`

Jika revoke gagal → transaction rollback → password tidak berubah → aman.
Jika update_password gagal → transaction rollback → token tetap valid → aman untuk retry.

Sama untuk `change_password`.

### D8 — Unit test strategy: mock-based unit tests + integration tests

Karena `AuthService` bergantung pada `Arc<dyn AuthRepository>` (trait), testing bisa dilakukan dengan mock repository:
- Buat `MockAuthRepository` di `#[cfg(test)]` module yang implementasi `AuthRepository`
- Test `AuthService::login`, `register`, `verify_otp`, `refresh`, `logout`, `resend_otp`, `forgot_password`, `reset_password`, `change_password`, `request_change_password_otp`, `suspend_account`, `suspend_accounts_bulk`
- Test handler response shapes (unit test dengan mock state)
- Test `PgAuthRepository` dengan `sqlx::test` (test database)

Target: ≥85% line coverage pada `auth-service` crate.

### D9 — Clean Architecture DIP: extract `TokenIssuer` dan `RateLimiter` traits

Buat dua trait di domain/application layer:
- `TokenIssuer`: `issue_access_token(user_id, email, status, role) -> Result<String>` — implementasi `JwtService`
- `RateLimiter`: `allow(key: &str) -> Result<bool>` — implementasi `OtpRateLimiter`

`AuthService` menyimpan `Arc<dyn TokenIssuer>` dan `Arc<dyn RateLimiter>`, bukan concrete types. Composition Root (`interface/mod.rs`) menghubungkan implementasi.

**Trade-off:** Menambah 2 file trait — small overhead, big testability gain.

### D10 — Code quality fixes

- `row_to_user()`: extract private method pada `PgAuthRepository` (SRP untuk row mapping)
- `sha256_hex()`: merge `hash_otp` + `hash_token` → satu fungsi
- `token_response_json()`: helper di handlers.rs untuk response shape standar
- `OTP_RATE_LIMIT_KEY_PREFIX`: konstan di rate_limit.rs
- Hapus `async-trait` dari Cargo.toml (tidak digunakan — repository trait sudah pakai native async fn)
- Sinkronkan `MAX_BULK_SUSPEND_USERS` dengan DTO: tambah komentar `// MUST match MAX_BULK_SUSPEND_USERS` pada validation attribute, atau gunakan `#[validate(custom = "...")]` yang referensi constant
- `DATABASE_URL` / `APP_PORT`: konformasi ke Config Standard

## Risks / Trade-offs

- **D2 (DELETE-first refresh)**: Jika `issue_tokens` gagal setelah DELETE, user kehilangan refresh token → harus re-login. Mitigasi: `issue_tokens` gagal hanya pada transient DB error (INSERT); user bisa retry login.
- **D5 (always EXPIRE)**: Sliding window bisa diperpanjang oleh attacker dengan request tepat sebelum window habis → max 2x window duration worst case. Mitigasi: MAX_REQUESTS=3 cukup rendah sehingga sliding window tidak signifikan.
- **D6 (dummy bcrypt)**: Dummy hash ter-hardcode di source code — bukan secret. Mitigasi: hash bcrypt adalah one-way, nilai ini hanya dipakai untuk timing constant, bukan untuk perbandingan kredensial.
- **D9 (DIP traits)**: Menambah 2 file trait — overhead kecil. Mitigasi: trait sangat tipis (1-2 method), tidak akan bertambah kompleks.

## Migration Plan

1. Semua perubahan bersifat internal `auth-service` — tidak ada migrasi DB, tidak ada perubahan API kontrak
2. Rollback: revert ke commit sebelum merge
3. Integration test di `rejki-app/tests/auth_integration_test.rs` harus tetap pass — jalankan sebelum merge

## Open Questions

- **Dummy bcrypt hash value**: `$2b$12$LJ3m4ys3Lk0TSwHCpNqrAOZBXK8mB3yZF5s0HVMCJzmVCAg8FwvKe` (dari seed migration) — cukup untuk dummy constant-time path? **Resolved: yes, any valid bcrypt hash works.**
- **Login rate limit threshold**: 5 percobaan / 15 menit per email — cukup generous? **Dikonfirmasi: reasonable default, bisa di-tune via env var.**
- **Unit test coverage measurement**: `cargo tarpaulin` atau `cargo llvm-cov`? **Diserahkan ke implementer.**
