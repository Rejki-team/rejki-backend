CREATE MATERIALIZED VIEW analytics.mv_engagement_stats AS
SELECT
    -- Chat
    (SELECT COUNT(*) FROM chat.messages WHERE created_at >= CURRENT_DATE - INTERVAL '7 days') AS messages_7d,
    (SELECT COUNT(*) FROM chat.messages WHERE created_at >= CURRENT_DATE - INTERVAL '30 days') AS messages_30d,
    (SELECT COUNT(DISTINCT conversation_id) FROM chat.messages WHERE created_at >= CURRENT_DATE - INTERVAL '7 days') AS conversations_active_7d,
    -- Notifikasi
    (SELECT COUNT(*) FROM notification.notifications WHERE created_at >= CURRENT_DATE - INTERVAL '7 days') AS notif_sent_7d,
    (SELECT COUNT(*) FROM notification.notifications WHERE read_at IS NOT NULL AND created_at >= CURRENT_DATE - INTERVAL '7 days') AS notif_read_7d,
    (SELECT COUNT(*) FROM notification.notifications WHERE created_at >= CURRENT_DATE - INTERVAL '30 days') AS notif_sent_30d,
    (SELECT COUNT(*) FROM notification.notifications WHERE read_at IS NOT NULL AND created_at >= CURRENT_DATE - INTERVAL '30 days') AS notif_read_30d;

CREATE UNIQUE INDEX idx_mv_engagement_stats ON analytics.mv_engagement_stats (messages_7d);
