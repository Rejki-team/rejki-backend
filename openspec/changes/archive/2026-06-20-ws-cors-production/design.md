## Context

CORS saat ini di `main.rs` menggunakan `allow_origin(Any)` — wildcard untuk semua environment. Ini diperlukan selama development (frontend lokal di port berbeda), tapi tidak aman untuk production. Security baseline memerlukan whitelist origin eksplisit.

## Goals / Non-Goals

**Goals:**
- Environment-aware CORS: development = longgar (`Any`), production = whitelist
- Env var `CORS_ALLOWED_ORIGINS` sebagai single source of truth untuk production whitelist
- Default whitelist: `https://rejki.id,https://app.rejki.id`
- `allow_credentials(true)` di production untuk future HttpOnly cookie support

**Non-Goals:**
- Tidak mengubah middleware stack selain CorsLayer
- Tidak mengubah behavior development — tetap `Any` seperti sekarang

## Decisions

### D1: Conditional di main.rs, bukan middleware terpisah
CorsLayer hanya dipasang sekali di main.rs. Logika kondisional: jika production dan CORS_ALLOWED_ORIGINS di-set, gunakan whitelist; jika development, gunakan `Any`. Tidak perlu middleware baru.

### D2: Default value langsung di AppConfig
`CORS_ALLOWED_ORIGINS` punya default (`rejki.id,app.rejki.id`) sehingga production bisa jalan tanpa env var — tapi tetap override-able via env var untuk staging/custom domain.

### D3: allow_credentials(true) hanya di production
`allow_credentials(true)` tidak kompatibel dengan `allow_origin(Any)` (akan panic di tower-http). Development tetap tanpa credentials agar bisa pakai `Any`. Production dengan whitelist mengaktifkan credentials untuk future HttpOnly refresh-token cookie.

## Risks / Trade-offs

| Risk | Mitigasi |
|---|---|
| Salah parse URL origin → panic startup | Parsing di AppConfig::from_env — fail-fast, bukan runtime |
| Lupa update whitelist saat deploy domain baru | Cukup set CORS_ALLOWED_ORIGINS env var — no code change |
| allow_credentials(true) + wildcard → panic | Conditional logic memastikan credentials tidak pernah aktif dengan Any |
