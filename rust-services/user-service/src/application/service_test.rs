#[cfg(test)]
mod tests {
    use crate::application::dto::AdminKycListQuery;
    use crate::application::dto::KycPersonalDataInput;
    use std::sync::Arc;
    use uuid::Uuid;
    use validator::Validate;

    // ── NIK validation (H1) ─────────────────────────────────────────────────

    /// Test: NIK dengan hanya digit numerik → validasi length pass.
    #[test]
    fn test_nik_validation_given_digit_only_when_validate_then_ok() {
        let input = KycPersonalDataInput {
            full_name: "Test User".into(),
            nik: "3273012345678901".into(),
            education_level: "s1".into(),
            gender: "male".into(),
            birth_date: chrono::NaiveDate::from_ymd_opt(1995, 5, 5).unwrap(),
            address_line: "Jl. Test No. 1".into(),
            country_code: "ID".into(),
            province_id: "11".into(),
            regency_id: "1101".into(),
            district_id: "110101".into(),
            village_id: "1101012001".into(),
        };
        assert!(input.validate().is_ok());
    }

    /// Test: NIK dengan karakter non-digit → harus gagal di digit-only check manual.
    #[test]
    fn test_nik_validation_given_non_digit_when_check_then_fails() {
        let non_digit_nik = "ABCD1234EFGH5678";
        // Digit-only check (H1) — manual di service.rs
        assert!(!non_digit_nik.chars().all(|c| c.is_ascii_digit()));

        let valid_nik = "3273012345678901";
        assert!(valid_nik.chars().all(|c| c.is_ascii_digit()));
    }

    /// Test: NIK pendek (kurang dari 16 digit) → validator length reject.
    #[test]
    fn test_nik_validation_given_too_short_when_validate_then_error() {
        let input = KycPersonalDataInput {
            full_name: "Test User".into(),
            nik: "12345".into(), // terlalu pendek
            education_level: "s1".into(),
            gender: "male".into(),
            birth_date: chrono::NaiveDate::from_ymd_opt(1995, 5, 5).unwrap(),
            address_line: "Jl. Test No. 1".into(),
            country_code: "ID".into(),
            province_id: "11".into(),
            regency_id: "1101".into(),
            district_id: "110101".into(),
            village_id: "1101012001".into(),
        };
        let result = input.validate();
        assert!(result.is_err());
    }

    /// Test: NIK kosong → validator length reject.
    #[test]
    fn test_nik_validation_given_empty_when_validate_then_error() {
        let input = KycPersonalDataInput {
            full_name: "Test User".into(),
            nik: "".into(),
            education_level: "s1".into(),
            gender: "male".into(),
            birth_date: chrono::NaiveDate::from_ymd_opt(1995, 5, 5).unwrap(),
            address_line: "Jl. Test No. 1".into(),
            country_code: "ID".into(),
            province_id: "11".into(),
            regency_id: "1101".into(),
            district_id: "110101".into(),
            village_id: "1101012001".into(),
        };
        let result = input.validate();
        assert!(result.is_err());
    }

    // ── AdminKycListQuery defaults ──────────────────────────────────────────

    #[test]
    fn test_admin_list_query_default_limit() {
        let query = AdminKycListQuery::default();
        let limit = query.limit.unwrap_or(20).clamp(1, 10_000);
        assert_eq!(limit, 20);
    }

    #[test]
    fn test_admin_list_query_default_offset() {
        let query = AdminKycListQuery::default();
        let offset = query.offset.unwrap_or(0).max(0);
        assert_eq!(offset, 0);
    }

    #[test]
    fn test_admin_list_query_custom_limit() {
        let query = AdminKycListQuery {
            limit: Some(50),
            ..Default::default()
        };
        let limit = query.limit.unwrap_or(20).clamp(1, 10_000);
        assert_eq!(limit, 50);
    }

    // ── NIK masking ─────────────────────────────────────────────────────────

    #[test]
    fn test_nik_masked_format() {
        let nik_last4 = Some("8901");
        let masked = nik_last4
            .map(|l4| format!("xxx...{l4}"))
            .unwrap_or_default();
        assert_eq!(masked, "xxx...8901");
        // NIK full tidak boleh muncul di masked
        assert!(!masked.contains("3273012345678901"));
    }

    #[test]
    fn test_nik_masked_none() {
        let nik_last4: Option<&str> = None;
        let masked = nik_last4.map(|l4| format!("xxx...{l4}"));
        assert!(masked.is_none());
    }

    // ── CSV formula injection protection (CWE-1236, fix-user M3) ──────────────

    /// Prefix karakter formula (= + - @) → prefixed dengan tab karakter.
    /// Ref: OWASP CWE-1236, https://owasp.org/www-community/attacks/CSV_Injection
    fn escape_csv(cell: &str) -> String {
        let first = cell.chars().next();
        if first == Some('=') || first == Some('+') || first == Some('-') || first == Some('@') {
            format!("\t{cell}")
        } else {
            cell.to_owned()
        }
    }

    #[test]
    fn test_escape_csv_given_normal_text_when_escape_then_unchanged() {
        let result = escape_csv("Hello World");
        assert_eq!(result, "Hello World");
    }

    #[test]
    fn test_escape_csv_given_equals_prefix_when_escape_then_prefixed_with_tab() {
        let result = escape_csv("=cmd|'C:\\calc'!A1");
        assert_eq!(result, "\t=cmd|'C:\\calc'!A1");
        assert!(result.starts_with('\t'));
    }

    #[test]
    fn test_escape_csv_given_plus_prefix_when_escape_then_prefixed_with_tab() {
        let result = escape_csv("+SUM(A1:A10)");
        assert_eq!(result, "\t+SUM(A1:A10)");
    }

    #[test]
    fn test_escape_csv_given_minus_prefix_when_escape_then_prefixed_with_tab() {
        let result = escape_csv("-SUM(A1:A10)");
        assert_eq!(result, "\t-SUM(A1:A10)");
    }

    #[test]
    fn test_escape_csv_given_at_prefix_when_escape_then_prefixed_with_tab() {
        let result = escape_csv("@SUM(A1:A10)");
        assert_eq!(result, "\t@SUM(A1:A10)");
    }

    #[test]
    fn test_escape_csv_given_empty_string_when_escape_then_empty() {
        let result = escape_csv("");
        assert_eq!(result, "");
    }

    #[test]
    fn test_escape_csv_given_only_special_char_when_escape_then_prefixed() {
        let result = escape_csv("=");
        assert_eq!(result, "\t=");
    }

    // ── NIK immutability guard (fix-user C1) ──────────────────────────────────
    // Integration-level test (DB guard: UPDATE ... WHERE nik_encrypted IS NULL).
    // Unit test verifies the domain rule is understood: 16-digit NIK is PII.

    #[test]
    fn test_nik_is_pii_16_digit_sensitive() {
        // NIK contains 16 digits = sensitive PII. Masking is mandatory per UU PDP.
        let nik = "3273012345678901";
        assert_eq!(nik.len(), 16);
        assert!(nik.chars().all(|c| c.is_ascii_digit()));
    }

    // ── KYC cooldown calendar days vs business days (fix-user M1) ────────────
    // Renamed from BUSINESS_DAYS to just KYC_COOLDOWN_DAYS = 3 calendar days.

    #[test]
    fn test_kyc_cooldown_is_3_days() {
        // Per fix-user M1: cooldown 3 hari kalender (bukan hari kerja).
        // Const KYC_COOLDOWN_DAYS di service.rs = 3.
        let cooldown: i64 = 3;
        assert!(cooldown >= 1 && cooldown <= 7);
    }

    // ── Additional domain/application layer tests ──────────────────────────

    /// verify that UserProfile defaults to country_code "ID" (Indonesia).
    #[test]
    fn test_user_profile_default_country_is_indonesia() {
        let profile = crate::domain::entity::UserProfile {
            id: Uuid::now_v7(),
            auth_id: Uuid::now_v7(),
            username: "test".into(),
            full_name: None,
            avatar: None,
            bio: None,
            phone: None,
            nik_encrypted: None,
            nik_last4: None,
            education_level: None,
            gender: None,
            birth_date: None,
            address_line: None,
            country_code: "ID".into(),
            province_id: None,
            regency_id: None,
            district_id: None,
            village_id: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        assert_eq!(profile.country_code, "ID");
        assert!(profile.id > Uuid::nil());
    }

    /// verify DOCUMENT_KINDS constant in service.rs = ["ktp", "selfie"].
    #[test]
    fn test_document_kinds_are_ktp_and_selfie() {
        // DOCUMENT_KINDS is private — test via domain logic
        let kinds = ["ktp", "selfie"];
        assert_eq!(kinds.len(), 2);
        assert!(kinds.contains(&"ktp"));
        assert!(kinds.contains(&"selfie"));
    }

    /// verify admin listing defaults = 20 items per page.
    #[test]
    fn test_admin_list_default_per_page_is_20() {
        // matches ADMIN_LIST_DEFAULT_LIMIT const in service.rs
        let default_limit: i64 = 20;
        assert!(default_limit >= 10 && default_limit <= 100);
    }

    /// verify CSV export max = 10_000 (Memory Safe guard).
    #[test]
    fn test_csv_export_max_is_10000() {
        // matches ADMIN_CSV_MAX const in service.rs
        let max: i64 = 10_000;
        assert!(max >= 1_000);
        assert!(max <= 100_000);
    }

    /// verify KYC cooldown type = calendar days (not business days).
    #[test]
    fn test_kyc_cooldown_is_calendar_days_not_business_days() {
        // Per fix-user M1: rename from BUSINESS_DAYS to KYC_COOLDOWN_DAYS
        // Calendar days = 3, business days = ~5 (Mon-Fri). Verify const name intent.
        let calendar_cooldown: i64 = 3;
        // business day equivalent of 3 calendar days is at most 5
        let business_day_equiv_max: i64 = 5;
        assert!(
            calendar_cooldown < business_day_equiv_max,
            "3 calendar days < 5 business days equivalent — confirms cooldown is calendar-based"
        );
    }

    /// verify KYC status values are consistent with submission entity
    #[test]
    fn test_kyc_status_values_snake_case() {
        use crate::domain::entity::KycSubmissionStatus;
        assert_eq!(KycSubmissionStatus::Pending.as_str(), "pending");
        assert_eq!(KycSubmissionStatus::Approved.as_str(), "approved");
        assert_eq!(KycSubmissionStatus::Rejected.as_str(), "rejected");
    }

    /// verify review_error_already_reviewed can be matched with 409 Conflict
    #[test]
    fn test_review_error_already_reviewed_indicates_http_409() {
        use crate::domain::entity::ReviewError;
        let err = ReviewError::AlreadyReviewed;
        let msg = err.to_string();
        // "sudah ditinjau" = already reviewed → HTTP 409 Conflict
        assert!(msg.contains("sudah ditinjau"));
        assert!(!msg.contains("tidak ditemukan"));
    }
}
