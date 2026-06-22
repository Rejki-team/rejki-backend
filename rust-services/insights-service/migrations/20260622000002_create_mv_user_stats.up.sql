CREATE MATERIALIZED VIEW analytics.mv_user_stats AS
SELECT
    COUNT(*) FILTER (WHERE deleted_at IS NULL) AS total_users,
    COUNT(*) FILTER (WHERE created_at >= CURRENT_DATE AND deleted_at IS NULL) AS new_users_today,
    COUNT(*) FILTER (WHERE created_at >= date_trunc('week', CURRENT_DATE) AND deleted_at IS NULL) AS new_users_week,
    COUNT(*) FILTER (WHERE created_at >= date_trunc('month', CURRENT_DATE) AND deleted_at IS NULL) AS new_users_month,
    COUNT(*) FILTER (WHERE status = 'active' AND deleted_at IS NULL) AS active_users,
    COUNT(*) FILTER (WHERE status IN ('active', 'pending_kyc') AND deleted_at IS NULL) AS verified_users,
    (SELECT COUNT(*) FROM auth.users WHERE created_at >= date_trunc('month', CURRENT_DATE - INTERVAL '1 month') AND created_at < date_trunc('month', CURRENT_DATE) AND deleted_at IS NULL) AS prev_month_users,
    -- Conversion funnel
    (SELECT COUNT(*) FROM auth.users WHERE deleted_at IS NULL) AS funnel_registered,
    (SELECT COUNT(*) FROM user_svc.profiles WHERE deleted_at IS NULL) AS funnel_profile_filled,
    (SELECT COUNT(DISTINCT profile_id) FROM user_svc.kyc_submissions WHERE deleted_at IS NULL) AS funnel_kyc_submitted,
    (SELECT COUNT(*) FROM user_svc.profiles WHERE kyc_status = 'approved' AND deleted_at IS NULL) AS funnel_kyc_approved
FROM auth.users;

-- Unique index for CONCURRENTLY refresh
CREATE UNIQUE INDEX idx_mv_user_stats ON analytics.mv_user_stats (total_users);
