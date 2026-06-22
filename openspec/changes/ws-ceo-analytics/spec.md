# Spec: CEO Analytics Endpoint (W3D-12)

## 1. Role Executive

### 1.1 Enum

```rust
pub enum Role {
    User = 20,
    UserVerified = 40,
    Moderator = 60,
    AdminIklan = 80,
    AdminUser = 80,
    Executive = 90,    // NEW
    SuperAdmin = 100,
}
```

### 1.2 Authorization

- `require_role(90)` untuk semua endpoint `/api/v1/insights/*`
- Executive **tidak berhak** mengakses admin endpoints (gate rank=80 exclusive untuk admin)
  - Guard: `is_admin_strict()` = `matches!(role, SuperAdmin | AdminIklan | AdminUser)`
  - Admin endpoints migrasi dari `rank() >= 80` ke `is_admin_strict()`

## 2. Service: insights-service

### 2.1 Materialized Views — Schema `analytics`

View di-refresh dengan `REFRESH MATERIALIZED VIEW CONCURRENTLY` (membutuhkan unique index).

#### mv_user_stats
```sql
SELECT
    COUNT(*) FILTER (WHERE deleted_at IS NULL) AS total_users,
    COUNT(*) FILTER (WHERE created_at >= CURRENT_DATE AND deleted_at IS NULL) AS new_users_today,
    COUNT(*) FILTER (WHERE created_at >= date_trunc('week', CURRENT_DATE) AND deleted_at IS NULL) AS new_users_week,
    COUNT(*) FILTER (WHERE created_at >= date_trunc('month', CURRENT_DATE) AND deleted_at IS NULL) AS new_users_month,
    COUNT(*) FILTER (WHERE status = 'active' AND deleted_at IS NULL) AS active_users,
    COUNT(*) FILTER (WHERE status IN ('active', 'pending_kyc') AND deleted_at IS NULL) AS verified_users,
    (SELECT COUNT(*) FROM auth.users WHERE created_at >= date_trunc('month', CURRENT_DATE - interval '1 month') AND created_at < date_trunc('month', CURRENT_DATE) AND deleted_at IS NULL) AS prev_month_users
FROM auth.users;
```

#### mv_iklan_stats
```sql
SELECT * FROM (
    -- Pekerjaan
    SELECT 'pekerjaan' AS vertikal,
        COUNT(*) FILTER (WHERE is_active AND deleted_at IS NULL) AS aktif,
        COUNT(*) FILTER (WHERE deleted_at IS NULL) AS total,
        COUNT(*) FILTER (WHERE created_at >= CURRENT_DATE - interval '7 days' AND deleted_at IS NULL) AS baru_7d,
        COUNT(*) FILTER (WHERE created_at >= CURRENT_DATE - interval '30 days' AND deleted_at IS NULL) AS baru_30d
    FROM iklan_pekerjaan.iklan_pekerjaan
    UNION ALL
    -- Pekerja
    SELECT 'pekerja' AS vertikal,
        COUNT(*) FILTER (WHERE is_active AND deleted_at IS NULL),
        COUNT(*) FILTER (WHERE deleted_at IS NULL),
        COUNT(*) FILTER (WHERE created_at >= CURRENT_DATE - interval '7 days' AND deleted_at IS NULL),
        COUNT(*) FILTER (WHERE created_at >= CURRENT_DATE - interval '30 days' AND deleted_at IS NULL)
    FROM iklan_pekerja.iklan_pekerja
    UNION ALL
    -- Barang Bekas
    SELECT 'barang_bekas',
        COUNT(*) FILTER (WHERE is_active AND is_sold = false AND deleted_at IS NULL),
        COUNT(*) FILTER (WHERE deleted_at IS NULL),
        COUNT(*) FILTER (WHERE created_at >= CURRENT_DATE - interval '7 days' AND deleted_at IS NULL),
        COUNT(*) FILTER (WHERE created_at >= CURRENT_DATE - interval '30 days' AND deleted_at IS NULL)
    FROM iklan_barang_bekas.iklan_barang_bekas
    UNION ALL
    -- Pelatihan
    SELECT 'pelatihan',
        COUNT(*) FILTER (WHERE is_active AND moderation_status = 'approved' AND deleted_at IS NULL),
        COUNT(*) FILTER (WHERE deleted_at IS NULL),
        COUNT(*) FILTER (WHERE created_at >= CURRENT_DATE - interval '7 days' AND deleted_at IS NULL),
        COUNT(*) FILTER (WHERE created_at >= CURRENT_DATE - interval '30 days' AND deleted_at IS NULL)
    FROM iklan_pelatihan.iklan_pelatihan
) AS stats;
```

#### mv_geo_stats
```sql
SELECT
    COALESCE(p.id, 'unknown') AS province_id,
    p.name AS province_name,
    COUNT(*) FILTER (WHERE ik.type = 'pekerjaan') AS iklan_pekerjaan,
    COUNT(*) FILTER (WHERE ik.type = 'pekerja') AS iklan_pekerja,
    COUNT(*) FILTER (WHERE ik.type = 'barang_bekas') AS iklan_barang_bekas,
    COUNT(*) FILTER (WHERE ik.type = 'pelatihan') AS iklan_pelatihan,
    COUNT(*) AS total_iklan,
    COUNT(*) FILTER (WHERE ik.created_at >= CURRENT_DATE - interval '30 days') AS baru_30d,
    -- User count from KYC profiles
    (SELECT COUNT(DISTINCT up.user_id) FROM user_svc.profiles up WHERE up.province_id = p.id AND up.deleted_at IS NULL) AS total_users
FROM (
    SELECT region_id, 'pekerjaan' AS type, created_at FROM iklan_pekerjaan.iklan_pekerjaan WHERE deleted_at IS NULL AND region_id IS NOT NULL
    UNION ALL
    SELECT region_id, 'pekerja', created_at FROM iklan_pekerja.iklan_pekerja WHERE deleted_at IS NULL AND region_id IS NOT NULL
    UNION ALL
    SELECT region_id, 'barang_bekas', created_at FROM iklan_barang_bekas.iklan_barang_bekas WHERE deleted_at IS NULL AND region_id IS NOT NULL
    UNION ALL
    SELECT region_id, 'pelatihan', created_at FROM iklan_pelatihan.iklan_pelatihan WHERE deleted_at IS NULL AND region_id IS NOT NULL
) ik
LEFT JOIN region.provinces p ON p.id = ik.region_id OR (LENGTH(ik.region_id) >= 2 AND p.id = LEFT(ik.region_id, 2))
GROUP BY p.id, p.name;
```

#### mv_engagement_stats
```sql
SELECT
    (SELECT COUNT(*) FROM chat.messages WHERE created_at >= CURRENT_DATE - interval '7 days') AS messages_7d,
    (SELECT COUNT(*) FROM chat.messages WHERE created_at >= CURRENT_DATE - interval '30 days') AS messages_30d,
    (SELECT COUNT(DISTINCT conversation_id) FROM chat.messages WHERE created_at >= CURRENT_DATE - interval '7 days') AS conversations_active_7d,
    (SELECT COUNT(*) FROM notification.notifications WHERE created_at >= CURRENT_DATE - interval '7 days') AS notif_sent_7d,
    (SELECT COUNT(*) FROM notification.notifications WHERE read_at IS NOT NULL AND created_at >= CURRENT_DATE - interval '7 days') AS notif_read_7d,
    (SELECT COUNT(*) FROM notification.notifications WHERE created_at >= CURRENT_DATE - interval '30 days') AS notif_sent_30d,
    (SELECT COUNT(*) FROM notification.notifications WHERE read_at IS NOT NULL AND created_at >= CURRENT_DATE - interval '30 days') AS notif_read_30d
```

### 2.2 Unique Index untuk CONCURRENTLY

Setiap MV butuh unique index agar `REFRESH MATERIALIZED VIEW CONCURRENTLY` bisa berjalan.

```sql
CREATE UNIQUE INDEX idx_mv_user_stats ON analytics.mv_user_stats (total_users);
CREATE UNIQUE INDEX idx_mv_iklan_stats ON analytics.mv_iklan_stats (vertikal);
CREATE UNIQUE INDEX idx_mv_geo_stats ON analytics.mv_geo_stats (province_id);
CREATE UNIQUE INDEX idx_mv_engagement_stats ON analytics.mv_engagement_stats (messages_7d);
```

### 2.3 Refresh Strategy

- **Auto**: tokio cron setiap 30 menit — `REFRESH MATERIALIZED VIEW CONCURRENTLY analytics.mv_*`
- **Manual**: `POST /api/v1/insights/refresh` (require_role(90))
- Fail-open: error di-log, tidak panic

## 3. REST API

### 3.1 Endpoints

Semua endpoint di bawah prefix `/api/v1/insights/` dengan auth `require_role(90)`.

#### GET /insights/users
Query: `?period=[today|week|month|all]`
Response:
```json
{
  "success": true,
  "data": {
    "total_users": 12345,
    "new_users_today": 12,
    "new_users_week": 89,
    "new_users_month": 345,
    "active_users": 8901,
    "verified_users": 5678,
    "growth_rate_mom": 5.2,
    "conversion_funnel": {
      "registered": 12345,
      "profile_filled": 9876,
      "kyc_submitted": 6543,
      "active": 5678
    }
  }
}
```

#### GET /insights/iklan
Query: `?period=[today|week|month|all]&vertikal=[pekerjaan|pekerja|barang_bekas|pelatihan|all]`
Response: per-vertikal stats

#### GET /insights/geo
Query: `?province_id=<id>&period=[today|week|month|all]`
Response: per-provinsi stats + supply-demand ratio

#### GET /insights/engagement
Query: `?period=[today|week|month|all]`
Response: chat + notification stats

#### GET /insights/canvassing
Response: ranking provinsi dengan composite score + rekomendasi

#### POST /insights/refresh
Body: (none)
Response: `{ "status": "ok", "message": "sedang memperbarui data analytics" }`

### 3.2 Canvassing Score Algorithm

Composite score (0-100):
- `iklan_active_ratio * 0.30` — Density iklan aktif per user
- `user_active_ratio * 0.25` — Tingkat partisipasi pengguna
- `gap_score * 0.25` — Kesenjangan supply-demand (inkar pekerja:pekerjaan = high supply; kebalikannya = high demand)
- `growth_score * 0.20` — Pertumbuhan MoM

Priority tiers:
- `>= 80`: **Sangat Prioritas** — rekomendasi canvassing segera
- `>= 60`: **Prioritas** — pertimbangkan uji coba
- `>= 40`: **Potensial** — monitor
- `< 40`: **Tunda** — data belum cukup

## 4. Caching

- In-memory: `HashMap<String, (Instant, Value)>` di-proteksi `Arc<RwLock<>>`
- TTL: 300 detik (5 menit)
- Cache key: `"{method}:{path}:{query_string}"`
- Invalidasi: expired TTL otomatis + `POST /insights/refresh` clear all

## 5. Arsitektur

Pola 4-layer Clean Architecture mengikuti service lain (referensi: `report-service`).

```
domain/entity.rs       — DTO hasil query materialized views
domain/repository.rs   — InsightsRepository trait
application/dto.rs     — Request/Response DTO untuk handler
application/service.rs — InsightsService: cache + repository orchestration
infrastructure/        — PgInsightsRepository (sqlx query views)
interface/mod.rs       — Router + AppState + require_role(90)
interface/handlers.rs  — 6 Axum handlers (4 get + 1 canvassing + 1 refresh)
```

Tidak ada client crate — service ini tidak perlu dikonsumsi service lain.

## 6. Security

- Seluruh endpoint di-gate `require_role(90)` — hanya Executive & SuperAdmin
- Input validation: parameter enum divalidasi, default safe
- Tidak ada mutation endpoint kecuali refresh MV (read-only)
- Stack trace tidak bocor ke client
