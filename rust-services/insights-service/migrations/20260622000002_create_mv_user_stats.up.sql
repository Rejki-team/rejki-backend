-- Catatan perbaikan (2026-09-21): view asli mengasumsikan kolom `deleted_at` di
-- `auth.users` dan `user_svc.profiles` — kolom itu tidak pernah ada di kedua tabel
-- (auth/user_svc tidak memakai soft-delete, beda dengan tabel iklan_* yang memakainya).
-- Juga memperbaiki nama tabel `user_svc.kyc_submissions` (plural, tidak ada) menjadi
-- `user_svc.kyc_submission` (singular, sesuai 20260611000001_extend_user_profiles),
-- dan `funnel_kyc_approved` yang mengacu kolom `profiles.kyc_status` (tidak ada —
-- status pengajuan KYC ada di `kyc_submission.status`, bukan di `profiles`).
CREATE MATERIALIZED VIEW analytics.mv_user_stats AS
SELECT
    COUNT(*) AS total_users,
    COUNT(*) FILTER (WHERE created_at >= CURRENT_DATE) AS new_users_today,
    COUNT(*) FILTER (WHERE created_at >= date_trunc('week', CURRENT_DATE)) AS new_users_week,
    COUNT(*) FILTER (WHERE created_at >= date_trunc('month', CURRENT_DATE)) AS new_users_month,
    COUNT(*) FILTER (WHERE status = 'active') AS active_users,
    COUNT(*) FILTER (WHERE status IN ('active', 'pending_kyc')) AS verified_users,
    (SELECT COUNT(*) FROM auth.users WHERE created_at >= date_trunc('month', CURRENT_DATE - INTERVAL '1 month') AND created_at < date_trunc('month', CURRENT_DATE)) AS prev_month_users,
    -- Conversion funnel
    (SELECT COUNT(*) FROM auth.users) AS funnel_registered,
    (SELECT COUNT(*) FROM user_svc.profiles) AS funnel_profile_filled,
    (SELECT COUNT(DISTINCT profile_id) FROM user_svc.kyc_submission) AS funnel_kyc_submitted,
    (SELECT COUNT(DISTINCT profile_id) FROM user_svc.kyc_submission WHERE status = 'approved') AS funnel_kyc_approved
FROM auth.users;

-- Unique index for CONCURRENTLY refresh
CREATE UNIQUE INDEX idx_mv_user_stats ON analytics.mv_user_stats (total_users);
