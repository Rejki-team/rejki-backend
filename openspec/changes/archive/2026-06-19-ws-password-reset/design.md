## Context

Password reset flow sudah diimplementasikan sebagai bagian dari change `extend-auth-service-onboarding` (US-05 / US-06). Flow ini memanfaatkan OTP infrastructure yang sudah ada dengan purpose `reset_password` dan `change_password`. Design ini mendokumentasikan arsitektur yang sudah berjalan.

## Goals / Non-Goals

**Goals:**
- Dokumentasi arsitektur forgot-password + reset-password + change-password
- Catat pattern atomik transaction yang sudah diimplementasikan
- Catat security properties (anti-enumeration, timing-safe, rate limit)

**Non-Goals:**
- Perubahan kode — semua sudah berjalan
- Integrasi SMS OTP (hanya email)

## Decisions

### D1: Reuse OTP Infrastructure (not new table)
OTP reset password menggunakan tabel `otp_verifications` yang sama dengan register. Hanya dibedakan oleh kolom `purpose` (`reset_password` / `change_password`). Tidak perlu tabel baru.

### D2: Anti-Enumeration pada forgot-password
`forgot_password()` selalu mengembalikan 200 OK terlepas dari apakah email terdaftar. Jika email tidak ditemukan, tidak ada OTP yang dikirim dan tidak ada error yang dibocorkan.

### D3: Rate Limit 3 req/15 menit per email
Menggunakan `OtpRateLimiter` dari W3A-02 dengan key `otp_req:reset_password:{email_hash}`. Fail-open: jika Redis down, request tetap diproses (logged dengan `warn!`).

### D4: Atomik Transaction — consume OTP + update password + revoke tokens
`consume_otp_and_update_password_transactional()` membungkus tiga operasi dalam satu transaction SQL: (1) consume OTP (DELETE RETURNING), (2) update password hash, (3) revoke semua refresh token. Jika salah satu gagal, semua rollback. Mencegah OTP reuse dan partial update.

### D5: Change-Password Wajib Autentikasi + OTP
`request_change_password_otp()` membutuhkan JWT valid (AuthClaims). OTP dikirim ke email terdaftar — user tidak perlu memasukkan email lagi (mengurangi friction).

## Risks / Trade-offs

| Risk | Mitigasi |
|---|---|
| OTP brute-force | MAX_OTP_ATTEMPTS = 5, OTP di-bump setiap gagal, auto-invalidate jika exceeded |
| Timing side-channel | bcrypt verify SELALU dijalankan dengan dummy hash bila user tak ditemukan |
| Rate limit bypass via multi-email | Rate limit di-key per email (bukan per IP) — anti spam OTP |
| OTP email leak via notification | Email dikirim via notification-service contract; OTP di-hash sebelum disimpan (SHA-256) |
