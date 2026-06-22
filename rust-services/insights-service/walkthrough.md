# Walkthrough: W3D-12 CEO Analytics

## Ringkasan

Implementasi CEO Analytics Endpoints (W3D-12) untuk memberikan dashboard analytics tingkat eksekutif kepada role Executive (rank 90) dan SuperAdmin (rank 100).

## Yang Telah Dibangun

### 1. Executive Role (rank 90)
- Variant `Executive` di `auth-service-client/src/lib.rs` — rank 90
- Method `is_admin_operator()` untuk exclude Executive dari admin endpoints
- Migration `20260622000001_add_executive_role` — alter CHECK constraint
- Middleware `require_admin_operator()` di `common-auth-mw`

### 2. Analitik Database (6 Migration Files)
- Schema `analytics`
- 4 Materialized Views:
  - `mv_user_stats` — total users, new users, active, conversion funnel
  - `mv_iklan_stats` — per-vertikal (pekerjaan, pekerja, barang_bekas, pelatihan)
  - `mv_geo_stats` — per-provinsi dengan supply-demand ratio + canvassing score
  - `mv_engagement_stats` — chat messages + notifikasi 7d/30d

### 3. Service Layer (4 Layer Architecture)
- **Domain**: Entity (UserStats, IklanStats, GeoStats, EngagementStats) + Repository trait
- **Infrastructure**: PgInsightsRepository dengan sqlx runtime queries
- **Application**: DTO conversion + InsightsService orchestrator + in-memory cache (5 min TTL)
- **Interface**: 6 endpoints (users, iklan, geo, engagement, canvassing, refresh) + refresh scheduler (30 min)

### 4. Unit Test
- 8 unit test di `dto_test.rs` — DTO conversion logic
- Semua PASS ✅

### 5. Integration Test (via Podman PostgreSQL)
- Setup schema + seed data + migration langsung di DB
- 12 test functions mencakup:
  - ✅ user_stats, iklan_stats, geo_stats, engagement, canvassing, refresh
  - ✅ Query parameter filtering (vertikal, province_id)
  - ✅ RBAC: executive ✅, super_admin ✅, user ❌ (403), anon ❌ (401)

### 6. Wiring
- `rust-services/Cargo.toml` — member
- `rejki-app/Cargo.toml` — dependency
- `rejki-app/src/main.rs` — nest `/insights`
- `rejki-app/src/openapi_insights_docs.rs` — Swagger docs

## Test Coverage
- **8 unit test** (DTO logic) ✅
- **12 integration test** (API + RBAC via Podman) ✅
- `cargo fmt --all` ✅
- `cargo clippy --workspace -- -D warnings` ✅
- `cargo check --workspace` ✅
