//! Re-export OtpRateLimiter dari common/rate-limit — backward compat untuk auth-service.
//!
//! OTP rate limit key prefix tetap dipertahankan di sini untuk backward compat.
//! Implementasi konkret sekarang di common/rate-limit.

/// Re-export constants dari common untuk backward compat.
pub use common_rate_limit::BISNIS_CREATE_TIGHT as MAX_REQUESTS;

/// Key prefix untuk OTP rate limiting (backward compat).
pub const OTP_RATE_LIMIT_KEY_PREFIX: &str = "otp_req";

// Note: auth-service menggunakan `allow()` untuk OTP yang memformat key sendiri —
// kompatibel dengan common-rate-limit karena method signature identik.
// Prefix `otp_req` tidak diterapkan oleh common; callers auth tetap pakai allow_raw.
pub use common_rate_limit::OtpRateLimiter;
