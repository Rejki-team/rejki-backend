#[cfg(test)]
mod tests {
    use crate::application::dto::UserStatsResponse;
    use crate::domain::entity::UserStats;

    #[test]
    fn test_user_stats_conversion_mom_positive() {
        let stats = UserStats {
            total_users: 1000,
            new_users_today: 10,
            new_users_week: 50,
            new_users_month: 200,
            active_users: 600,
            verified_users: 400,
            prev_month_users: 150,
            funnel_registered: 1000,
            funnel_profile_filled: 800,
            funnel_kyc_submitted: 500,
            funnel_kyc_approved: 400,
        };
        let resp = UserStatsResponse::from(stats);
        assert_eq!(resp.total_users, 1000);
        assert!(resp.growth_rate_mom > 0.0);
        assert_eq!(resp.conversion_funnel.registered, 1000);
        assert_eq!(resp.conversion_funnel.active, 400);
    }

    #[test]
    fn test_user_stats_conversion_mom_negative() {
        let stats = UserStats {
            total_users: 1000,
            new_users_today: 5,
            new_users_week: 30,
            new_users_month: 80,
            active_users: 600,
            verified_users: 400,
            prev_month_users: 200,
            funnel_registered: 1000,
            funnel_profile_filled: 800,
            funnel_kyc_submitted: 500,
            funnel_kyc_approved: 400,
        };
        let resp = UserStatsResponse::from(stats);
        assert_eq!(resp.total_users, 1000);
        // 80 prev: 200, (80-200)/200*100 = -60.0
        assert_eq!(resp.growth_rate_mom, -60.0);
    }

    #[test]
    fn test_user_stats_prev_month_zero() {
        let stats = UserStats {
            total_users: 100,
            new_users_today: 1,
            new_users_week: 5,
            new_users_month: 10,
            active_users: 50,
            verified_users: 30,
            prev_month_users: 0,
            funnel_registered: 100,
            funnel_profile_filled: 80,
            funnel_kyc_submitted: 50,
            funnel_kyc_approved: 30,
        };
        let resp = UserStatsResponse::from(stats);
        assert_eq!(resp.total_users, 100);
        // prev 0 -> max(1), growth = (10-1)/1*100 = 900.0
        assert_eq!(resp.growth_rate_mom, 900.0);
    }

    #[test]
    fn test_iklan_stats_barang_bekas_persen_terjual() {
        use crate::application::dto::IklanStatsResponse;
        use crate::domain::entity::IklanStats;

        let stats = IklanStats {
            vertikal: "barang_bekas".into(),
            aktif: 80,
            total: 100,
            baru_7d: 10,
            baru_30d: 30,
            terjual: 25,
            suspended: 2,
        };
        let resp = IklanStatsResponse::from(stats);
        assert_eq!(resp.vertikal, "barang_bekas");
        assert_eq!(resp.persen_terjual, Some(25.0)); // 25/100 * 100 = 25.0
    }

    #[test]
    fn test_iklan_stats_non_barang_no_persen() {
        use crate::application::dto::IklanStatsResponse;
        use crate::domain::entity::IklanStats;

        let stats = IklanStats {
            vertikal: "pekerjaan".into(),
            aktif: 50,
            total: 100,
            baru_7d: 5,
            baru_30d: 20,
            terjual: 0,
            suspended: 1,
        };
        let resp = IklanStatsResponse::from(stats);
        assert_eq!(resp.vertikal, "pekerjaan");
        assert_eq!(resp.persen_terjual, None);
    }

    #[test]
    fn test_geo_stats_priority_tier() {
        use crate::application::dto::GeoStatsResponse;
        use crate::domain::entity::GeoStats;

        let make = |score: i32, name: &str| -> GeoStatsResponse {
            GeoStatsResponse::from(GeoStats {
                province_id: "test".into(),
                province_name: name.into(),
                iklan_pekerjaan: 100,
                iklan_pekerja: 50,
                iklan_barang_bekas: 30,
                iklan_pelatihan: 20,
                total_iklan: 200,
                baru_30d: 50,
                total_users: 5000,
                supply_demand_ratio: 2.0,
                canvassing_score: score,
            })
        };
        assert_eq!(make(85, "Jabar").priority_tier, "Sangat Prioritas");
        assert_eq!(make(65, "Jateng").priority_tier, "Prioritas");
        assert_eq!(make(45, "Jatim").priority_tier, "Potensial");
        assert_eq!(make(25, "Banten").priority_tier, "Tunda");
    }

    #[test]
    fn test_engagement_stats_read_ratio() {
        use crate::application::dto::EngagementStatsResponse;
        use crate::domain::entity::EngagementStats;

        let stats = EngagementStats {
            messages_7d: 1000,
            messages_30d: 4000,
            conversations_active_7d: 150,
            notif_sent_7d: 800,
            notif_read_7d: 400,
            notif_sent_30d: 3000,
            notif_read_30d: 1500,
        };
        let resp = EngagementStatsResponse::from(stats);
        assert_eq!(resp.notif_read_ratio_7d, Some(50.0)); // 400/800*100 = 50.0
        assert_eq!(resp.messages_7d, 1000);
    }

    #[test]
    fn test_engagement_stats_zero_sent() {
        use crate::application::dto::EngagementStatsResponse;
        use crate::domain::entity::EngagementStats;

        let stats = EngagementStats {
            messages_7d: 0,
            messages_30d: 0,
            conversations_active_7d: 0,
            notif_sent_7d: 0,
            notif_read_7d: 0,
            notif_sent_30d: 0,
            notif_read_30d: 0,
        };
        let resp = EngagementStatsResponse::from(stats);
        assert_eq!(resp.notif_read_ratio_7d, None);
    }
}
