## Tasks — ws-password-reset

> **Status:** ✅ SELESAI (06/2026) — semua task sudah diimplementasikan dari change `extend-auth-service-onboarding` (US-05 / US-06).

### Backend (Rust + Axum)

- [x] DTO: `ForgotPasswordInput`, `ResetPasswordInput`, `ChangePasswordInput` (di `application/dto.rs`)
- [x] Domain: `OtpPurpose::ResetPassword`, `OtpPurpose::ChangePassword` variant + validator (di `domain/entity.rs`)
- [x] Service: `forgot_password()` — anti-enumeration, rate limit 3/15m, OTP email (di `application/service.rs`)
- [x] Service: `reset_password()` — atomik consume OTP + update password + revoke tokens (di `application/service.rs`)
- [x] Service: `request_change_password_otp()` — authed, OTP email (di `application/service.rs`)
- [x] Service: `change_password()` — atomik consume OTP + update password + revoke OTHER tokens (di `application/service.rs`)
- [x] Repository: `consume_otp_and_update_password_transactional()` — transactional atomic operation (di `infrastructure/pg_repository.rs`)
- [x] Handler: `forgot_password`, `reset_password`, `request_change_password_otp`, `change_password` (di `interface/handlers.rs`)
- [x] Route: `POST /forgot-password`, `POST /reset-password`, `POST /change-password/otp`, `POST /change-password` (di `interface/mod.rs`)
- [x] OpenAPI: `ForgotPasswordDocRequest`, `ResetPasswordDocRequest`, `ChangePasswordDocRequest` + path annotations (di `rejki-app/src/openapi.rs`)
- [x] Audit event: `AuditEvent::PasswordReset`, `AuditEvent::PasswordChanged`, `OtpSent { purpose: "reset_password" | "change_password" }`
- [x] Rate limit: integrasi `OtpRateLimiter` (W3A-02) untuk forgot-password dan change-password-otp

### Testing

- [x] DTO validation: reset_password_input_validates, change_password_input_validates
- [x] Mocked AuthService tests via inline `MockAuthRepository`
- [x] Atomic transactional guard via `consume_otp_and_update_password_transactional`
- [x] OTP purpose validation for `reset_password` / `change_password`

### Dokumentasi

- [x] OpenAPI path annotations (gated APP_ENV=development)
- [x] Openspec proposal + design + spec + tasks
- [ ] Sync spec ke main `openspec/specs/password-reset/spec.md`
- [ ] Archive change
