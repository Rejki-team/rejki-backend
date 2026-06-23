use crate::domain::entity::{EngagementStats, GeoStats, IklanStats, UserStats};
use serde::{Deserialize, Serialize};

/// Konstanta nama kueri parameter.
pub mod query_period {
    pub const TODAY: &str = "today";
    pub const WEEK: &str = "week";
    pub const MONTH: &str = "month";
    pub const ALL: &str = "all";
}

/// Query params untuk filter insights.
#[derive(Debug, Deserialize)]
pub struct InsightsQuery {
    pub period: Option<String>,
    pub province_id: Option<String>,
    pub vertikal: Option<String>,
}

/// Response wrapper untuk statistik pengguna.
#[derive(Debug, Serialize, Deserialize)]
pub struct UserStatsResponse {
    pub total_users: i64,
    pub new_users_today: i64,
    pub new_users_week: i64,
    pub new_users_month: i64,
    pub active_users: i64,
    pub verified_users: i64,
    pub growth_rate_mom: f64,
    pub conversion_funnel: ConversionFunnel,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConversionFunnel {
    pub registered: i64,
    pub profile_filled: i64,
    pub kyc_submitted: i64,
    pub active: i64,
}

impl From<UserStats> for UserStatsResponse {
    fn from(s: UserStats) -> Self {
        let prev = s.prev_month_users.max(1);
        let growth_rate_mom = ((s.new_users_month as f64 - prev as f64) / prev as f64) * 100.0;
        Self {
            total_users: s.total_users,
            new_users_today: s.new_users_today,
            new_users_week: s.new_users_week,
            new_users_month: s.new_users_month,
            active_users: s.active_users,
            verified_users: s.verified_users,
            growth_rate_mom: (growth_rate_mom * 10.0).round() / 10.0,
            conversion_funnel: ConversionFunnel {
                registered: s.funnel_registered,
                profile_filled: s.funnel_profile_filled,
                kyc_submitted: s.funnel_kyc_submitted,
                active: s.funnel_kyc_approved,
            },
        }
    }
}

/// Response per vertikal.
#[derive(Debug, Serialize, Deserialize)]
pub struct IklanStatsResponse {
    pub vertikal: String,
    pub aktif: i64,
    pub total: i64,
    pub baru_7d: i64,
    pub baru_30d: i64,
    pub terjual: i64,
    pub suspended: i64,
    pub persen_terjual: Option<f64>,
}

impl From<IklanStats> for IklanStatsResponse {
    fn from(s: IklanStats) -> Self {
        let persen_terjual = if s.total > 0 && s.vertikal == "barang_bekas" {
            Some(((s.terjual as f64 / s.total as f64) * 100.0 * 10.0).round() / 10.0)
        } else {
            None
        };
        Self {
            vertikal: s.vertikal,
            aktif: s.aktif,
            total: s.total,
            baru_7d: s.baru_7d,
            baru_30d: s.baru_30d,
            terjual: s.terjual,
            suspended: s.suspended,
            persen_terjual,
        }
    }
}

/// Response geografis per provinsi.
#[derive(Debug, Serialize, Deserialize)]
pub struct GeoStatsResponse {
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
    pub priority_tier: String,
}

impl From<GeoStats> for GeoStatsResponse {
    fn from(s: GeoStats) -> Self {
        let priority_tier = match s.canvassing_score {
            80.. => "Sangat Prioritas",
            60.. => "Prioritas",
            40.. => "Potensial",
            _ => "Tunda",
        };
        Self {
            province_id: s.province_id,
            province_name: s.province_name,
            iklan_pekerjaan: s.iklan_pekerjaan,
            iklan_pekerja: s.iklan_pekerja,
            iklan_barang_bekas: s.iklan_barang_bekas,
            iklan_pelatihan: s.iklan_pelatihan,
            total_iklan: s.total_iklan,
            baru_30d: s.baru_30d,
            total_users: s.total_users,
            supply_demand_ratio: s.supply_demand_ratio,
            canvassing_score: s.canvassing_score,
            priority_tier: priority_tier.to_string(),
        }
    }
}

/// Response engagement.
#[derive(Debug, Serialize, Deserialize)]
pub struct EngagementStatsResponse {
    pub messages_7d: i64,
    pub messages_30d: i64,
    pub conversations_active_7d: i64,
    pub notif_sent_7d: i64,
    pub notif_read_7d: i64,
    pub notif_sent_30d: i64,
    pub notif_read_30d: i64,
    pub notif_read_ratio_7d: Option<f64>,
}

impl From<EngagementStats> for EngagementStatsResponse {
    fn from(s: EngagementStats) -> Self {
        let notif_read_ratio_7d = if s.notif_sent_7d > 0 {
            Some(((s.notif_read_7d as f64 / s.notif_sent_7d as f64) * 100.0 * 10.0).round() / 10.0)
        } else {
            None
        };
        Self {
            messages_7d: s.messages_7d,
            messages_30d: s.messages_30d,
            conversations_active_7d: s.conversations_active_7d,
            notif_sent_7d: s.notif_sent_7d,
            notif_read_7d: s.notif_read_7d,
            notif_sent_30d: s.notif_sent_30d,
            notif_read_30d: s.notif_read_30d,
            notif_read_ratio_7d,
        }
    }
}

/// Ringkasan canvassing.
#[derive(Debug, Serialize, Deserialize)]
pub struct CanvassingSummaryResponse {
    pub total_provinces_analyzed: usize,
    pub high_priority_count: usize,
    pub medium_priority_count: usize,
    pub avg_canvassing_score: f64,
}

/// Response /insights/canvassing.
#[derive(Debug, Serialize, Deserialize)]
pub struct CanvassingResponse {
    pub provinces: Vec<GeoStatsResponse>,
    pub summary: CanvassingSummaryResponse,
}

/// Response refresh.
#[derive(Debug, Serialize, Deserialize)]
pub struct RefreshResponse {
    pub status: String,
    pub message: String,
}
