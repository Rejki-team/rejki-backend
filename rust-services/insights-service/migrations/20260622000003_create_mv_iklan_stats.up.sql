CREATE MATERIALIZED VIEW analytics.mv_iklan_stats AS
WITH iklan_data AS (
    SELECT 'pekerjaan' AS vertikal,
        COUNT(*) FILTER (WHERE is_active AND deleted_at IS NULL) AS aktif,
        COUNT(*) FILTER (WHERE deleted_at IS NULL) AS total,
        COUNT(*) FILTER (WHERE created_at >= CURRENT_DATE - INTERVAL '7 days' AND deleted_at IS NULL) AS baru_7d,
        COUNT(*) FILTER (WHERE created_at >= CURRENT_DATE - INTERVAL '30 days' AND deleted_at IS NULL) AS baru_30d,
        0::bigint AS terjual,
        COUNT(*) FILTER (WHERE moderation_status IN ('suspended_temp', 'suspended_permanent') AND deleted_at IS NULL) AS suspended
    FROM iklan_pekerjaan.iklan
    UNION ALL
    SELECT 'pekerja' AS vertikal,
        COUNT(*) FILTER (WHERE is_active AND deleted_at IS NULL),
        COUNT(*) FILTER (WHERE deleted_at IS NULL),
        COUNT(*) FILTER (WHERE created_at >= CURRENT_DATE - INTERVAL '7 days' AND deleted_at IS NULL),
        COUNT(*) FILTER (WHERE created_at >= CURRENT_DATE - INTERVAL '30 days' AND deleted_at IS NULL),
        0::bigint,
        COUNT(*) FILTER (WHERE moderation_status IN ('suspended_temp', 'suspended_permanent') AND deleted_at IS NULL)
    FROM iklan_pekerja.iklan
    UNION ALL
    -- Catatan perbaikan (2026-09-21): `iklan_barang_bekas.iklan` TIDAK punya kolom
    -- `is_active`/`is_sold` — status ketersediaan disimpan sebagai
    -- `availability_status` ('tersedia' / 'sudah_diambil').
    SELECT 'barang_bekas' AS vertikal,
        COUNT(*) FILTER (WHERE availability_status = 'tersedia' AND deleted_at IS NULL),
        COUNT(*) FILTER (WHERE deleted_at IS NULL),
        COUNT(*) FILTER (WHERE created_at >= CURRENT_DATE - INTERVAL '7 days' AND deleted_at IS NULL),
        COUNT(*) FILTER (WHERE created_at >= CURRENT_DATE - INTERVAL '30 days' AND deleted_at IS NULL),
        COUNT(*) FILTER (WHERE availability_status = 'sudah_diambil' AND deleted_at IS NULL),
        COUNT(*) FILTER (WHERE moderation_status IN ('suspended_temp', 'suspended_permanent') AND deleted_at IS NULL)
    FROM iklan_barang_bekas.iklan
    UNION ALL
    -- Catatan perbaikan (2026-09-21): `moderation_status = 'approved'` bukan nilai
    -- valid (CHECK constraint hanya 'active'/'suspended_temp'/'suspended_permanent');
    -- disederhanakan jadi `is_active` saja, konsisten dgn vertikal pekerjaan/pekerja.
    SELECT 'pelatihan' AS vertikal,
        COUNT(*) FILTER (WHERE is_active AND deleted_at IS NULL),
        COUNT(*) FILTER (WHERE deleted_at IS NULL),
        COUNT(*) FILTER (WHERE created_at >= CURRENT_DATE - INTERVAL '7 days' AND deleted_at IS NULL),
        COUNT(*) FILTER (WHERE created_at >= CURRENT_DATE - INTERVAL '30 days' AND deleted_at IS NULL),
        0::bigint,
        COUNT(*) FILTER (WHERE moderation_status IN ('suspended_temp', 'suspended_permanent') AND deleted_at IS NULL)
    FROM iklan_pelatihan.iklan
)
SELECT * FROM iklan_data;

CREATE UNIQUE INDEX idx_mv_iklan_stats ON analytics.mv_iklan_stats (vertikal);
