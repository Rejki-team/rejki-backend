CREATE UNIQUE INDEX IF NOT EXISTS idx_mv_geo_stats ON analytics.mv_geo_stats (province_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_mv_engagement_stats ON analytics.mv_engagement_stats (messages_7d);
