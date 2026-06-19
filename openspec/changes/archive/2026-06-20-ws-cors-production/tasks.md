## Tasks — ws-cors-production

### Backend (Rust + Axum)

- [x] `common/config/src/lib.rs` — tambah field `cors_allowed_origins: Vec<String>` dengan default `["https://rejki.id", "https://app.rejki.id"]`
- [x] `rejki-app/src/main.rs` — conditional CorsLayer: development → `Any`, production → whitelist dari `cfg.cors_allowed_origins` + `allow_credentials(true)`
- [x] `.env.example` — tambah `CORS_ALLOWED_ORIGINS` dengan placeholder

### Dokumentasi

- [x] `docs/config-standard.html` — tambah `CORS_ALLOWED_ORIGINS` ke katalog variabel shared
- [x] `docs/security-baseline.html` — update implementasi section CORS
- [x] `docs/implementation-plan-phase-3.html` — W3C-08 card plan → done
- [x] `docs/index.html` — tambah row W3C-08
- [x] `docs/brainstorm/phase-3-hardening-analysis.html` — gap A4 P2 → ✅ DONE
- [x] Sync spec ke main `openspec/specs/cors-policy/spec.md`
- [x] Archive change
