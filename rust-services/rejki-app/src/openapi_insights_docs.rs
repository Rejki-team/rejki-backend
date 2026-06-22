/// Respons statistik pengguna (insights).
#[derive(Debug, Serialize, ToSchema)]
pub struct UserStatsDocResponse {
    #[schema(example = 12345)]
    pub total_users: i64,
    #[schema(example = 12)]
    pub new_users_today: i64,
    #[schema(example = 89)]
    pub new_users_week: i64,
    #[schema(example = 345)]
    pub new_users_month: i64,
    #[schema(example = 8901)]
    pub active_users: i64,
    #[schema(example = 5678)]
    pub verified_users: i64,
    #[schema(example = 5.2)]
    pub growth_rate_mom: f64,
}

/// Respons iklan stats per vertikal (insights).
#[derive(Debug, Serialize, ToSchema)]
pub struct IklanStatsDocResponse {
    #[schema(example = "pekerjaan")]
    pub vertikal: String,
    #[schema(example = 150)]
    pub aktif: i64,
    #[schema(example = 200)]
    pub total: i64,
    #[schema(example = 10)]
    pub baru_7d: i64,
    #[schema(example = 45)]
    pub baru_30d: i64,
    #[schema(example = 0)]
    pub terjual: i64,
    #[schema(example = 5)]
    pub suspended: i64,
}

/// Respons geo stats per provinsi (insights).
#[derive(Debug, Serialize, ToSchema)]
pub struct GeoStatsDocResponse {
    #[schema(example = "32")]
    pub province_id: String,
    #[schema(example = "Jawa Barat")]
    pub province_name: String,
    #[schema(example = 80)]
    pub iklan_pekerjaan: i64,
    #[schema(example = 40)]
    pub iklan_pekerja: i64,
    #[schema(example = 30)]
    pub iklan_barang_bekas: i64,
    #[schema(example = 10)]
    pub iklan_pelatihan: i64,
    #[schema(example = 160)]
    pub total_iklan: i64,
    #[schema(example = 45)]
    pub baru_30d: i64,
    #[schema(example = 5000)]
    pub total_users: i64,
    #[schema(example = 2.0)]
    pub supply_demand_ratio: f64,
    #[schema(example = 87)]
    pub canvassing_score: i32,
    #[schema(example = "Sangat Prioritas")]
    pub priority_tier: String,
}

/// Respons engagement stats (insights).
#[derive(Debug, Serialize, ToSchema)]
pub struct EngagementStatsDocResponse {
    #[schema(example = 1200)]
    pub messages_7d: i64,
    #[schema(example = 5000)]
    pub messages_30d: i64,
    #[schema(example = 150)]
    pub conversations_active_7d: i64,
    #[schema(example = 800)]
    pub notif_sent_7d: i64,
    #[schema(example = 400)]
    pub notif_read_7d: i64,
    #[schema(example = 3500)]
    pub notif_sent_30d: i64,
    #[schema(example = 1800)]
    pub notif_read_30d: i64,
}

/// Respons canvassing (insights).
#[derive(Debug, Serialize, ToSchema)]
pub struct CanvassingDocResponse {
    pub provinces: Vec<GeoStatsDocResponse>,
    pub summary: CanvassingSummaryDocResponse,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CanvassingSummaryDocResponse {
    #[schema(example = 34)]
    pub total_provinces_analyzed: usize,
    #[schema(example = 5)]
    pub high_priority_count: usize,
    #[schema(example = 12)]
    pub medium_priority_count: usize,
    #[schema(example = 62.0)]
    pub avg_canvassing_score: f64,
}

/// Respons refresh insights.
#[derive(Debug, Serialize, ToSchema)]
pub struct RefreshDocResponse {
    #[schema(example = "ok")]
    pub status: String,
    #[schema(example = "Data analytics sedang diperbarui")]
    pub message: String,
}

// ── Insights Path Docs ─────────────────────────────────────────────────────────

/// GET /api/v1/insights/users — statistik pengguna (executive).
#[utoipa::path(get, path = "/api/v1/insights/users", tag = "insights",
    responses(
        (status = 200, description = "Statistik pengguna", body = UserStatsDocResponse),
        (status = 403, description = "Role tidak cukup — minimal executive (rank 90)"),
    )
)]
#[allow(dead_code)]
fn insights_users_doc() {}

/// GET /api/v1/insights/iklan — statistik iklan per vertikal (executive).
#[utoipa::path(get, path = "/api/v1/insights/iklan", tag = "insights",
    params(
        ("vertikal" = Option<String>, Query, description = "Filter vertikal: pekerjaan|pekerja|barang_bekas|pelatihan"),
    ),
    responses(
        (status = 200, description = "Statistik iklan per vertikal", body = Vec<IklanStatsDocResponse>),
        (status = 403, description = "Role tidak cukup — minimal executive (rank 90)"),
    )
)]
#[allow(dead_code)]
fn insights_iklan_doc() {}

/// GET /api/v1/insights/geo — sebaran geografis iklan (executive).
#[utoipa::path(get, path = "/api/v1/insights/geo", tag = "insights",
    params(
        ("province_id" = Option<String>, Query, description = "Filter provinsi"),
    ),
    responses(
        (status = 200, description = "Sebaran geografis per provinsi", body = Vec<GeoStatsDocResponse>),
        (status = 403, description = "Role tidak cukup — minimal executive (rank 90)"),
    )
)]
#[allow(dead_code)]
fn insights_geo_doc() {}

/// GET /api/v1/insights/engagement — volume chat & notifikasi (executive).
#[utoipa::path(get, path = "/api/v1/insights/engagement", tag = "insights",
    responses(
        (status = 200, description = "Statistik engagement chat & notifikasi", body = EngagementStatsDocResponse),
        (status = 403, description = "Role tidak cukup — minimal executive (rank 90)"),
    )
)]
#[allow(dead_code)]
fn insights_engagement_doc() {}

/// GET /api/v1/insights/canvassing — ranking prioritas wilayah (executive).
#[utoipa::path(get, path = "/api/v1/insights/canvassing", tag = "insights",
    responses(
        (status = 200, description = "Ranking prioritas canvassing per provinsi + ringkasan", body = CanvassingDocResponse),
        (status = 403, description = "Role tidak cukup — minimal executive (rank 90)"),
    )
)]
#[allow(dead_code)]
fn insights_canvassing_doc() {}

/// POST /api/v1/insights/refresh — refresh materialized views (executive).
#[utoipa::path(post, path = "/api/v1/insights/refresh", tag = "insights",
    responses(
        (status = 200, description = "Refresh MV dimulai; cache dibersihkan", body = RefreshDocResponse),
        (status = 403, description = "Role tidak cukup — minimal executive (rank 90)"),
    )
)]
#[allow(dead_code)]
fn insights_refresh_doc() {}
