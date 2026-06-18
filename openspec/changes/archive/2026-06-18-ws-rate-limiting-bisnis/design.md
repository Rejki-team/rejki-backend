# Design: ws-rate-limiting-bisnis

## Context

Saat ini hanya `auth-service` yang punya rate limiter — `OtpRateLimiter` mengimplementasikan `RateLimiter` trait di `auth-service/src/domain/rate_limit.rs`. Implementasi menggunakan Redis Lua script (atomic INCR + conditional EXPIRE), fail-open (bila Redis down → allow). Trait ini hanya bisa diakses oleh auth-service karena berada di dalam crate-nya.

Phase 2 mengimplementasikan 10+ service bisnis tanpa rate limiting sama sekali. Untuk Phase 3 hardening, semua endpoint publik harus dilindungi rate limiter. Pola yang sama (Redis Lua, fail-open) sudah battle-tested di auth-service dan harus diperluas ke semua service.

Keputusan arsitektur: **extract trait ke common crate**, bukan copy-paste. CLAUDE.md §3: "Antar service → lewat `*-service-client` trait". Tapi RateLimiter bukan domain client — ini cross-cutting concern seperti `common/errors` dan `common/tracing-setup`. Maka tempat yang tepat adalah `common/rate-limit`.

## Goals / Non-Goals

**Goals:**
- Extract `RateLimiter` trait + `OtpRateLimiter` impl ke `common/rate-limit` crate
- Inject `Arc<dyn RateLimiter>` ke 7 service: 4 iklan, report, chat, notification
- Rate limit pada endpoint create/tindakan (POST) dengan 30 req/menit per user
- Rate limit global default-deny di rejki-app (100 req/menit per IP) untuk defense-in-depth
- Response 429 dengan `Retry-After` header dan body error standar `RATE_LIMITED`
- Backward compatible — auth-service tetap pakai trait yang sama via re-export

**Non-Goals:**
- Tidak mengubah rate limit behavior auth-service (OTP, login) — tetap existing
- Tidak mengubah endpoint GET/list (read-only tidak perlu rate limit ketat, cukup global layer)
- Tidak implementasi tower-governor — pakai Redis Lua untuk konsistensi dengan auth-service
- Tidak menambah dependency baru di production (Redis sudah ada)

## Decisions

### D1 — Crate placement: `common/rate-limit` (bukan `auth-service-client`)

**Keputusan:** Buat `common/rate-limit` crate baru.

**Rasionale:**
- RateLimiter BUKAN domain client — ini cross-cutting concern seperti tracing/errors.
- `auth-service-client` hanya untuk trait komunikasi antar service (AuthClient). Menaruh RateLimiter di sana melanggar single responsibility.
- `common/rate-limit` sejajar dengan `common/errors`, `common/tracing-setup`, `common/crypto` — semua cross-cutting utility.
- Auth-service tetap bisa menggunakannya via dependency `common/rate-limit`.

**Alternatif ditolak:**
- Taruh di `auth-service-client`: melanggar SRP, membingungkan (client domain = kontrak komunikasi, bukan utility).
- Taruh di `common/errors`: rate limiting bukan error handling.

### D2 — Trait tetap `async_trait` (untuk `dyn` dispatch)

**Keputusan:** Trait `RateLimiter` tetap pakai `#[async_trait]`.

**Rasionale:**
- Injection via `Arc<dyn RateLimiter>` — butuh trait object. Native async fn in trait belum mendukung `dyn` dispatch tanpa `async-trait` crate.
- Konsisten dengan `AuthClient` trait yang juga pakai `#[async_trait]` untuk alasan yang sama (§4.1 CLAUDE.md).
- `async-trait` sudah workspace dependency.

### D3 — Satu instance `OtpRateLimiter` di-share via `Arc`

**Keputusan:** Satu `Arc<OtpRateLimiter>` dibuat di `rejki-app/main.rs`, di-inject ke semua service.

**Rasionale:**
- Redis connection pool di dalam `OtpRateLimiter` sudah thread-safe (multiplexed connection).
- Tidak perlu satu instance per service — rate limit key sudah namespaced by service/purpose.
- Pattern sama dengan `Arc<JwtService>` yang di-share ke auth-service.
- Wiring di Composition Root: `let rate_limiter = Arc::new(OtpRateLimiter::from_env());`

### D4 — Key namespace per service

**Keputusan:** Rate limit key format: `rl:{service}:{purpose}:{user_id}`

**Rasionale:**
- Mencegah collision antar service (mis. create iklan vs create report).
- `purpose` membedakan endpoint dalam satu service (mis. create vs send_message).
- Konsisten dengan format auth-service: `otp_req:{purpose}:{user_key}`.

Contoh:
- `rl:iklan_pekerjaan:create:{user_id}` — 30 req/15 menit
- `rl:report:create:{user_id}` — 10 req/15 menit
- `rl:chat:send_message:{user_id}` — 30 req/menit

### D5 — Rate limit config per endpoint

| Service | Purpose | Max Req | Window |
|---|---|---|---|
| iklan-pekerjaan | create | 30 | 15 menit |
| iklan-pekerja | create | 30 | 15 menit |
| iklan-barang-bekas | create | 30 | 15 menit |
| iklan-pelatihan | create | 20 | 15 menit |
| report | create | 10 | 15 menit |
| chat | send_message | 30 | 1 menit |
| notification | send | 20 | 1 menit |
| corporate-comms | create_article | 10 | 15 menit |
| **global** | **per_ip** | **100** | **1 menit** |

### D6 — Response format

```
HTTP/1.1 429 Too Many Requests
Retry-After: 60

{
  "success": false,
  "error": {
    "code": "RATE_LIMITED",
    "message": "Terlalu banyak permintaan. Silakan coba lagi dalam 60 detik."
  },
  "request_id": "..."
}
```

Konsisten dengan `security-baseline.html` §Rate Limiting.

### D7 — Global middleware layer (defense-in-depth)

**Keputusan:** Tambah `RateLimitLayer` di rejki-app sebagai middleware global sebelum service router.

**Rasionale:**
- Defense-in-depth: meskipun service lupa inject rate limiter, global layer tetap membatasi.
- 100 req/menit per IP — cukup longgar untuk operasi normal, cukup ketat untuk mitigasi DoS.
- Diterapkan sebagai axum middleware (bukan tower-governor — tetap pakai Redis Lua untuk konsistensi).

### D8 — Urutan implementasi

1. Buat `common/rate-limit` crate + pindahkan trait + impl
2. Update `auth-service` — re-export dari common, verifikasi test tetap PASS
3. Inject ke `iklan-pekerjaan-service` — prototype pertama, verify pola
4. Inject ke 3 iklan lain + report + chat + notification + corporate-comms
5. Tambah global middleware di rejki-app
6. Unit test + integration test (429 response)

## Risks / Trade-offs

| Risk | Mitigation |
|---|---|
| Redis single point of failure → semua service diblokir | Fail-open pattern: bila Redis tidak tersedia, rate limiter return `true` (allow). Traffic tidak diblokir. |
| Durasi Redis latency nambah di setiap request POST | Redis connection multiplexed, INCR+EXPIRE Lua script atomic ~1ms. Dampak minimal. |
| Key collision antar service | Namespace key `rl:{service}:{purpose}:{user_id}` mencegah collision. |
| Global IP-based rate limit memblokir shared network (NAT) | Threshold 100 req/menit cukup longgar. Bisa dinaikkan via env var jika perlu. |
| Service lupa inject rate limiter | Global layer di rejki-app sebagai safety net. |
