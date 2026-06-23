CREATE MATERIALIZED VIEW analytics.mv_iklan_stats AS
WITH iklan_data AS (
    SELECT 'pekerjaan' AS vertikal,
        COUNT(*) FILTER (WHERE is_active AND deleted_at IS NULL) AS aktif,
        COUNT(*) FILTER (WHERE deleted_at IS NULL) AS total,
        COUNT(*) FILTER (WHERE created_at >= CURRENT_DATE - INTERVAL '7 days' AND deleted_at IS NULL) AS baru_7d,
        COUNT(*) FILTER (WHERE created_at >= CURRENT_DATE - INTERVAL '30 days' AND deleted_at IS NULL) AS baru_30d,
        0::bigint AS terjual,
        COUNT(*) FILTER (WHERE moderation_status = 'suspended' AND deleted_at IS NULL) AS suspended
    FROM iklan_pekerjaan.iklan_pekerjaan
    UNION ALL
    SELECT 'pekerja' AS vertikal,
        COUNT(*) FILTER (WHERE is_active AND deleted_at IS NULL),
        COUNT(*) FILTER (WHERE deleted_at IS NULL),
        COUNT(*) FILTER (WHERE created_at >= CURRENT_DATE - INTERVAL '7 days' AND deleted_at IS NULL),
        COUNT(*) FILTER (WHERE created_at >= CURRENT_DATE - INTERVAL '30 days' AND deleted_at IS NULL),
        0::bigint,
        COUNT(*) FILTER (WHERE moderation_status = 'suspended' AND deleted_at IS NULL)
    FROM iklan_pekerja.iklan_pekerja
    UNION ALL
    SELECT 'barang_bekas' AS vertikal,
        COUNT(*) FILTER (WHERE is_active AND is_sold = false AND deleted_at IS NULL),
        COUNT(*) FILTER (WHERE deleted_at IS NULL),
        COUNT(*) FILTER (WHERE created_at >= CURRENT_DATE - INTERVAL '7 days' AND deleted_at IS NULL),
        COUNT(*) FILTER (WHERE created_at >= CURRENT_DATE - INTERVAL '30 days' AND deleted_at IS NULL),
        COUNT(*) FILTER (WHERE is_sold = true AND deleted_at IS NULL),
        COUNT(*) FILTER (WHERE moderation_status = 'suspended' AND deleted_at IS NULL)
    FROM iklan_barang_bekas.iklan_barang_bekas
    UNION ALL
    SELECT 'pelatihan' AS vertikal,
        COUNT(*) FILTER (WHERE is_active AND moderation_status = 'approved' AND deleted_at IS NULL),
        COUNT(*) FILTER (WHERE deleted_at IS NULL),
        COUNT(*) FILTER (WHERE created_at >= CURRENT_DATE - INTERVAL '7 days' AND deleted_at IS NULL),
        COUNT(*) FILTER (WHERE created_at >= CURRENT_DATE - INTERVAL '30 days' AND deleted_at IS NULL),
        0::bigint,
        COUNT(*) FILTER (WHERE moderation_status = 'suspended' AND deleted_at IS NULL)
    FROM iklan_pelatihan.iklan_pelatihan
)
SELECT * FROM iklan_data;

CREATE UNIQUE INDEX idx_mv_iklan_stats ON analytics.mv_iklan_stats (vertikal);
