use chrono::{DateTime, Datelike, Utc, Weekday};

/// Tambah `days` HARI KERJA (Senin-Jumat) ke `start` — PRD §6.10 "7 hari kerja",
/// eksplisit bukan kalender biasa. Hari libur nasional TIDAK diperhitungkan (di
/// luar scope — butuh kalender libur terpisah, tidak diminta spec ini).
pub fn add_business_days(start: DateTime<Utc>, days: i64) -> DateTime<Utc> {
    let mut result = start;
    let mut remaining = days;
    while remaining > 0 {
        result += chrono::Duration::days(1);
        if !matches!(result.weekday(), Weekday::Sat | Weekday::Sun) {
            remaining -= 1;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_add_business_days_given_monday_when_add_7_then_skips_2_weekends() {
        // Senin 2026-09-21 + 7 hari kerja = Rabu 2026-09-30 (melewati 1 akhir pekan).
        let start = Utc.with_ymd_and_hms(2026, 9, 21, 0, 0, 0).unwrap();
        let due = add_business_days(start, 7);
        assert_eq!(due.weekday(), Weekday::Wed);
        assert_eq!(
            due.date_naive(),
            Utc.with_ymd_and_hms(2026, 9, 30, 0, 0, 0)
                .unwrap()
                .date_naive()
        );
    }

    #[test]
    fn test_add_business_days_given_friday_when_add_1_then_lands_monday() {
        let start = Utc.with_ymd_and_hms(2026, 9, 25, 0, 0, 0).unwrap();
        let due = add_business_days(start, 1);
        assert_eq!(due.weekday(), Weekday::Mon);
    }
}
