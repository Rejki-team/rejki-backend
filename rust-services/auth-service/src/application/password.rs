use validator::ValidationError;

/// Kebijakan password (K4): min 8 karakter, mengandung minimal 1 huruf kapital,
/// 1 angka, dan 1 karakter spesial.
pub fn validate_password_strength(password: &str) -> Result<(), ValidationError> {
    let len_ok = password.chars().count() >= 8;
    let has_upper = password.chars().any(|c| c.is_ascii_uppercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    let has_special = password.chars().any(|c| !c.is_alphanumeric());

    if len_ok && has_upper && has_digit && has_special {
        return Ok(());
    }

    let mut err = ValidationError::new("password_weak");
    err.message = Some(
        "password minimal 8 karakter dan harus memuat huruf kapital, angka, dan karakter spesial"
            .into(),
    );
    Err(err)
}

/// Persetujuan T&C wajib `true` (US-01).
pub fn validate_tos_accepted(accepted: &bool) -> Result<(), ValidationError> {
    if *accepted {
        Ok(())
    } else {
        let mut err = ValidationError::new("tos_required");
        err.message = Some("persetujuan syarat & ketentuan wajib".into());
        Err(err)
    }
}

/// Validasi format E.164 sederhana: diawali '+' lalu 8–15 digit. `None` dianggap valid (opsional).
pub fn validate_e164(phone: &str) -> Result<(), ValidationError> {
    let bytes = phone.as_bytes();
    let valid = bytes.first() == Some(&b'+') && {
        let digits = &phone[1..];
        let n = digits.chars().count();
        (8..=15).contains(&n) && digits.chars().all(|c| c.is_ascii_digit())
    };

    if valid {
        Ok(())
    } else {
        let mut err = ValidationError::new("phone_invalid");
        err.message = Some("nomor telepon harus format E.164, mis. +6281234567890".into());
        Err(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_strength_rules() {
        assert!(validate_password_strength("Abcd123!").is_ok());
        assert!(validate_password_strength("short1!").is_err()); // < 8
        assert!(validate_password_strength("alllower1!").is_err()); // tanpa kapital
        assert!(validate_password_strength("NoDigits!").is_err()); // tanpa angka
        assert!(validate_password_strength("NoSpecial1").is_err()); // tanpa spesial
    }

    #[test]
    fn e164_rules() {
        assert!(validate_e164("+6281234567890").is_ok());
        assert!(validate_e164("081234567890").is_err()); // tanpa +
        assert!(validate_e164("+123").is_err()); // terlalu pendek
        assert!(validate_e164("+62812abc").is_err()); // ada huruf
    }
}
