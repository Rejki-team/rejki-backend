//! Entitas domain untuk hasil query materialized views.
//! Menggunakan `sqlx::FromRow` untuk runtime query_as.

/// Statistik pengguna dari materialized view mv_user_stats.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserStats {
    pub total_users: i64,
    pub new_users_today: i64,
    pub new_users_week: i64,
    pub new_users_month: i64,
    pub active_users: i64,
    pub verified_users: i64,
    pub prev_month_users: i64,
    pub funnel_registered: i64,
    pub funnel_profile_filled: i64,
    pub funnel_kyc_submitted: i64,
    pub funnel_kyc_approved: i64,
}

/// Statistik iklan per vertikal dari mv_iklan_stats.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct IklanStats {
    pub vertikal: String,
    pub aktif: i64,
    pub total: i64,
    pub baru_7d: i64,
    pub baru_30d: i64,
    pub terjual: i64,
    pub suspended: i64,
}

/// Statistik geografis per provinsi dari mv_geo_stats.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct GeoStats {
    pub province_id: String,
    pub province_name: String,
    pub iklan_pekerjaan: i64,
    pub iklan_pekerja: i64,
    pub iklan_barang_bekas: i64,
    pub iklan_pelatihan: i64,
    pub total_iklan: i64,
    pub baru_30d: i64,
    pub total_users: i64,
    pub supply_demand_ratio: f64,
    pub canvassing_score: i32,
}

/// Statistik engagement dari mv_engagement_stats.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct EngagementStats {
    pub messages_7d: i64,
    pub messages_30d: i64,
    pub conversations_active_7d: i64,
    pub notif_sent_7d: i64,
    pub notif_read_7d: i64,
    pub notif_sent_30d: i64,
    pub notif_read_30d: i64,
}
