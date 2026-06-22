CREATE MATERIALIZED VIEW analytics.mv_geo_stats AS
WITH iklan_region AS (
    SELECT region_id, 'pekerjaan' AS type, created_at
    FROM iklan_pekerjaan.iklan_pekerjaan
    WHERE deleted_at IS NULL AND region_id IS NOT NULL
    UNION ALL
    SELECT region_id, 'pekerja', created_at
    FROM iklan_pekerja.iklan_pekerja
    WHERE deleted_at IS NULL AND region_id IS NOT NULL
    UNION ALL
    SELECT region_id, 'barang_bekas', created_at
    FROM iklan_barang_bekas.iklan_barang_bekas
    WHERE deleted_at IS NULL AND region_id IS NOT NULL
    UNION ALL
    SELECT region_id, 'pelatihan', created_at
    FROM iklan_pelatihan.iklan_pelatihan
    WHERE deleted_at IS NULL AND region_id IS NOT NULL
),
province_stats AS (
    SELECT
        LEFT(ir.region_id, 2) AS province_code,
        p.id AS province_id,
        p.name AS province_name,
        COUNT(*) FILTER (WHERE ir.type = 'pekerjaan') AS iklan_pekerjaan,
        COUNT(*) FILTER (WHERE ir.type = 'pekerja') AS iklan_pekerja,
        COUNT(*) FILTER (WHERE ir.type = 'barang_bekas') AS iklan_barang_bekas,
        COUNT(*) FILTER (WHERE ir.type = 'pelatihan') AS iklan_pelatihan,
        COUNT(*) AS total_iklan,
        COUNT(*) FILTER (WHERE ir.created_at >= CURRENT_DATE - INTERVAL '30 days') AS baru_30d
    FROM iklan_region ir
    LEFT JOIN region.provinces p ON p.id = LEFT(ir.region_id, 2)
    GROUP BY LEFT(ir.region_id, 2), p.id, p.name
)
SELECT
    COALESCE(province_id, 'unknown') AS province_id,
    COALESCE(province_name, 'Luar Wilayah') AS province_name,
    iklan_pekerjaan,
    iklan_pekerja,
    iklan_barang_bekas,
    iklan_pelatihan,
    total_iklan,
    baru_30d,
    -- User count from KYC profiles (per provinsi)
    COALESCE((SELECT COUNT(DISTINCT up.user_id)
        FROM user_svc.profiles up
        WHERE up.province_id = province_code AND up.deleted_at IS NULL), 0) AS total_users,
    -- Supply-demand ratio: pekerjaan / (pekerja + 1) untuk hindari div-by-zero
    CASE
        WHEN (iklan_pekerja + 1) = 0 THEN iklan_pekerjaan::numeric
        ELSE ROUND(iklan_pekerjaan::numeric / (GREATEST(iklan_pekerja, 1)::numeric), 2)
    END AS supply_demand_ratio,
    -- Canvassing score composite
    ROUND(
        (LEAST(total_iklan::numeric / 100.0, 1.0) * 0.30 +
         LEAST((SELECT COUNT(*) FROM auth.users WHERE deleted_at IS NULL)::numeric / 10000.0, 1.0) * 0.25 +
         LEAST(ABS(iklan_pekerjaan - iklan_pekerja)::numeric / GREATEST(iklan_pekerjaan + iklan_pekerja, 1), 1.0) * 0.25 +
         LEAST(baru_30d::numeric / GREATEST(total_iklan, 1), 1.0) * 0.20
        ) * 100, 0
    )::integer AS canvassing_score
FROM province_stats;

CREATE UNIQUE INDEX idx_mv_geo_stats ON analytics.mv_geo_stats (province_id);
