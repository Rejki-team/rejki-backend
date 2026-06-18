#[cfg(test)]
mod tests {
    use crate::domain::entity::*;
    use std::str::FromStr;

    // ── KycSubmissionStatus ─────────────────────────────────────────────────

    #[test]
    fn test_kyc_status_as_str_pending() {
        assert_eq!(KycSubmissionStatus::Pending.as_str(), "pending");
    }

    #[test]
    fn test_kyc_status_as_str_approved() {
        assert_eq!(KycSubmissionStatus::Approved.as_str(), "approved");
    }

    #[test]
    fn test_kyc_status_as_str_rejected() {
        assert_eq!(KycSubmissionStatus::Rejected.as_str(), "rejected");
    }

    #[test]
    fn test_kyc_status_from_str_valid_pending() {
        let parsed: KycSubmissionStatus = "pending".parse().unwrap();
        assert_eq!(parsed, KycSubmissionStatus::Pending);
    }

    #[test]
    fn test_kyc_status_from_str_valid_approved() {
        let parsed: KycSubmissionStatus = "approved".parse().unwrap();
        assert_eq!(parsed, KycSubmissionStatus::Approved);
    }

    #[test]
    fn test_kyc_status_from_str_valid_rejected() {
        let parsed: KycSubmissionStatus = "rejected".parse().unwrap();
        assert_eq!(parsed, KycSubmissionStatus::Rejected);
    }

    #[test]
    fn test_kyc_status_from_str_invalid_returns_err() {
        assert!("invalid".parse::<KycSubmissionStatus>().is_err());
        assert!("".parse::<KycSubmissionStatus>().is_err());
        assert!("PENDING".parse::<KycSubmissionStatus>().is_err());
    }

    #[test]
    fn test_kyc_status_from_str_and_as_str_roundtrip() {
        for status in [
            KycSubmissionStatus::Pending,
            KycSubmissionStatus::Approved,
            KycSubmissionStatus::Rejected,
        ] {
            let s = status.as_str();
            let parsed: KycSubmissionStatus = s.parse().unwrap();
            assert_eq!(parsed, status);
        }
    }

    // ── DocumentAccessAction ────────────────────────────────────────────────

    #[test]
    fn test_document_access_action_as_str() {
        assert_eq!(DocumentAccessAction::UploadIssued.as_str(), "upload_issued");
        assert_eq!(DocumentAccessAction::Commit.as_str(), "commit");
        assert_eq!(DocumentAccessAction::ReadIssued.as_str(), "read_issued");
    }

    // ── ReviewError ─────────────────────────────────────────────────────────

    #[test]
    fn test_review_error_display_already_reviewed() {
        let err = ReviewError::AlreadyReviewed;
        let msg = err.to_string();
        assert!(msg.contains("sudah ditinjau"));
    }

    #[test]
    fn test_review_error_display_not_found() {
        let err = ReviewError::NotFound;
        let msg = err.to_string();
        assert!(msg.contains("tidak ditemukan"));
    }

    #[test]
    fn test_review_error_from_anyhow() {
        let inner = anyhow::anyhow!("db error");
        let err = ReviewError::Other(inner);
        let msg = err.to_string();
        assert!(msg.contains("db error"));
    }
}
