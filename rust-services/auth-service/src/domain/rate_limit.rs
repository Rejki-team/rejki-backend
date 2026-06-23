// Re-export from common/rate-limit — backward compat.
// Application layer depends on this trait (DIP), not on concrete OtpRateLimiter.
pub use common_rate_limit::RateLimiter;
