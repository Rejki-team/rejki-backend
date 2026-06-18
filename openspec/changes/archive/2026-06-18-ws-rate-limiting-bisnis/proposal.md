## Why

Phase 2 mengimplementasikan semua endpoint bisnis (4 iklan, report, chat, notifikasi, corporate-comms) tanpa rate limiting. Hanya auth-service yang punya `OtpRateLimiter` (Redis Lua atomik) untuk proteksi OTP brute-force. Akibatnya: seluruh endpoint publik rentan DoS, credential spraying, dan spam. Phase 3 W3A harus menutup gap ini sebelum production deployment. Rate Limiter trait + implementation sudah ada di auth-service — tinggal extract ke lokasi reusable dan inject ke semua service.

## What Changes

- **Extract `RateLimiter` trait** dari `auth-service/domain/rate_limit.rs` ke `common/rate-limit` crate (shared library) — memungkinkan semua service meng-inject rate limiter tanpa dependency ke auth-service.
- **Extract `OtpRateLimiter` implementation** ke `common/rate-limit` (Redis Lua atomik, fail-open — identik dengan yang ada).
- **Inject `RateLimiter` ke service application layer**: semua service yang punya endpoint POST publik.
- **Tambah rate limit check di handler create/tindakan**: 4 iklan (POST create), report (POST create), chat (POST send message), notification (POST send).
- **Priority endpoints**: POST create iklan (4 vertikal), POST /reports, POST /chat/messages. Default: 30 req/menit per user untuk create, 100 req/menit per user untuk list/read.
- **Response standar**: HTTP 429 Too Many Requests + `Retry-After` header + body error `RATE_LIMITED`.
- **Middleware default-deny di rejki-app**: global rate limit layer (100 req/menit per IP) sebagai defense-in-depth. Endpoint auth tetap pakai rate limiter existing (tidak diubah).

## Capabilities

### New Capabilities
- `rate-limiting-bisnis`: Rate limiting untuk endpoint publik bisnis — create iklan, buat aduan, kirim pesan, kirim notifikasi. Mencakup trait extraction, shared implementation, injection ke 7 service, response 429, dan unit/integration test.

### Modified Capabilities
- `auth-rate-limiting`: Trait `RateLimiter` dipindahkan ke `common/rate-limit` agar bisa di-share. Auth-service tetap menggunakan trait yang sama via re-export — tidak ada perubahan behavior.

## Impact

- **Kode**: extract trait + impl dari `auth-service/src/domain/rate_limit.rs` dan `auth-service/src/infrastructure/rate_limit.rs` ke `common/rate-limit/src/lib.rs`. Update 7 service untuk inject `Arc<dyn RateLimiter>`.
- **rejki-app wiring**: tambah inisialisasi `OtpRateLimiter::from_env()` dan inject ke semua service router.
- **Cargo workspace**: tambah `common/rate-limit` member + dependency `redis`, `async-trait`, `tokio`.
- **auth-service**: `rate_limit.rs` menjadi re-export dari `common/rate-limit`. Domain trait tetap di `domain/rate_limit.rs` sebagai re-export untuk backward compat.
- **Testing**: unit test dengan `MockRateLimiter` (sudah ada pattern `DenyingRateLimiter` di auth-service tests). Integration test 429 response.
- **Tidak ada breaking API changes**. Tidak ada migrasi DB. Tidak ada dependency runtime baru (Redis sudah existing).
- **Risiko**: rendah. Rate limiter sudah battle-tested di auth-service. Pattern extract + inject sudah established (mirror dengan `AuthClient` trait extraction).
