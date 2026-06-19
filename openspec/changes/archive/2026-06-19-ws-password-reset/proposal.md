## Why

Password reset flow adalah fitur keamanan esensial untuk pengguna yang lupa password. Meskipun OTP infrastructure sudah ada (register, change-password), endpoint `forgot-password` dan `reset-password` belum di-dokumentasikan sebagai change resmi dan perlu openspec tracking.

## What Changes

- **Tidak ada kode baru** — seluruh implementasi sudah ada dari change `extend-auth-service-onboarding` (US-05 / US-06)
- Dokumen openspec resmi untuk melacak W3B-07 sebagai P1 Wave 3B
- Archive change setelah openspec selesai

## Capabilities

### New Capabilities
- `password-reset`: Forgot-password (request OTP via email) + reset-password (OTP verification + new password) + change-password (authed + OTP)

### Modified Capabilities
- (none — implementasi sudah ada dan tidak mengubah requirements)

## Impact

- **auth-service**: Handler `forgot_password`, `reset_password`, `request_change_password_otp`, `change_password` (sudah ada di `handlers.rs`)
- **auth-service**: Service methods `forgot_password()`, `reset_password()`, `request_change_password_otp()`, `change_password()` di `service.rs`
- **auth-service**: Repository method `consume_otp_and_update_password_transactional()` (atomik)
- **OpenAPI**: `ForgotPasswordDocRequest`, `ResetPasswordDocRequest`, `ChangePasswordDocRequest` di `openapi.rs`
- **Audit**: `AuditEvent::PasswordReset`, `AuditEvent::PasswordChanged`, `AuditEvent::OtpSent { purpose: "reset_password" | "change_password" }`
