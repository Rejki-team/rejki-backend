# Tasks: CEO Analytics Endpoint (W3D-12)

## Task List

### Task 1: Role Executive (rank 90)
- [x] Tambah `Executive` variant di `auth-service-client/src/lib.rs` — rank 90
- [x] Migration baru: alter CHECK constraint tambah `'executive'`
- [x] Migration rollback
- [x] Update `is_admin()` → `is_admin_strict()` untuk executive exclusion
- [x] Update test fixtures

### Task 2: Migration — Schema analytics + 4 Materialized Views
- [x] `20260622000001_create_schema_analytics.up.sql` — CREATE SCHEMA analytics
- [x] `20260622000002_create_mv_user_stats.up.sql` — mv dari auth.users
- [x] `20260622000003_create_mv_iklan_stats.up.sql` — mv dari 4 iklan schema
- [x] `20260622000004_create_mv_geo_stats.up.sql` — mv geo + user distribution
- [x] `20260622000005_create_mv_engagement_stats.up.sql` — mv chat + notification
- [x] `20260622000006_create_idx_analytics.up.sql` — unique indexes untuk CONCURRENTLY
- [x] Down migrations untuk semua

### Task 3: Domain Layer — Entity + Repository
- [x] `src/domain/entity.rs` — UserStats, IklanStats, GeoStats, EngagementStats, CanvassingResult
- [x] `src/domain/repository.rs` — InsightsRepository trait (query methods + refresh)

### Task 4: Infrastructure — PgInsightsRepository
- [x] `src/infrastructure/pg_repository.rs` — sqlx query semua materialized views
- [x] Implement `REFRESH MATERIALIZED VIEW CONCURRENTLY`
- [x] Implement `warn_slow!` macro

### Task 5: Application — DTO + Service
- [x] `src/application/dto.rs` — Request/Response DTOs
- [x] `src/application/service.rs` — InsightsService (orchestrasi + cache)
- [x] In-memory cache: Arc<RwLock<HashMap<String, (Instant, serde_json::Value)>>>
- [x] TTL 300 detik

### Task 6: Interface — Router + Handlers
- [x] `src/interface/mod.rs` — Router 5 endpoints + require_role(90)
- [x] `src/interface/handlers.rs` — 6 handler functions

### Task 7: Refresh Scheduler
- [x] `src/refresh.rs` — RefreshCoordinator: tokio interval 30 menit
- [x] Wire di insights-service: spawn di `router()` atau di `rejki-app`

### Task 8: Wiring — Cargo.toml + rejki-app
- [x] `rust-services/Cargo.toml` — + insights-service di members
- [x] `rust-services/insights-service/Cargo.toml`
- [x] `rust-services/rejki-app/Cargo.toml` — + dep insights-service
- [x] `rust-services/rejki-app/src/main.rs` — + nest /insights + spawn scheduler

### Task 9: Swagger — openapi.rs
- [x] Mirror DTO untuk setiap response
- [x] Path annotation untuk 6 endpoints
- [x] Tag `insights` baru

### Task 10: Unit Test
- [x] MockInsightsRepository
- [x] InsightsService test (5 test functions)
- [x] Cache behavior test
- [x] Router RBAC test (executive vs non-executive)
- [x] Refresh coordinator test

### Task 11: Integration Test (via Podman PostgreSQL)
- [x] Setup Podman machine + PG container
- [x] Setup test schema + seed data
- [x] Jalankan migration (6 MV files)
- [x] Integration test: user_stats, iklan_stats, geo_stats, engagement, canvassing, refresh
- [x] Integration test: RBAC gating (executive ✅, user ❌, super_admin ✅, anon ❌)
- [x] Integration test: query parameter filtering

### Task 12: Finalisasi
- [x] `cargo fmt --all`
- [x] `cargo clippy --workspace -- -D warnings`
- [x] `cargo check --workspace`
- [x] `cargo test -p insights-service` (8 unit test + integration test)
- [x] `cargo sqlx prepare --workspace`
- [x] Update `implementation-plan-phase-3.html` status W3D-12
- [x] Update dokumentasi openspec & /docs
