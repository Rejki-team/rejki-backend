## Requirement: Forgot-password dengan OTP reset_password

Sistem SHALL menyediakan endpoint `POST /api/v1/auth/forgot-password` yang mengirim OTP dengan purpose `reset_password` ke email terdaftar. Demi anti-enumeration, sistem SHALL selalu mengembalikan 200 OK terlepas dari apakah email terdaftar.

#### Scenario: Email terdaftar — OTP reset_password terkirim
- **GIVEN** pengguna memiliki akun terdaftar dengan email `user@rejki.id`
- **WHEN** pengguna meminta forgot-password untuk email tersebut
- **THEN** sistem mengirim OTP `reset_password` via email dan membalas 200 OK

#### Scenario: Email tidak terdaftar — silent success
- **GIVEN** email `ghost@example.com` tidak terdaftar
- **WHEN** pengguna meminta forgot-password untuk email tersebut
- **THEN** sistem membalas 200 OK tanpa mengirim OTP (anti-enumeration)

#### Scenario: Rate limit terlampaui — tetap 200 OK
- **GIVEN** pengguna telah meminta forgot-password 3× dalam 15 menit
- **WHEN** pengguna meminta lagi
- **THEN** sistem tetap membalas 200 OK tanpa mengirim OTP

## Requirement: Reset-password dengan OTP + password baru

Sistem SHALL menyediakan endpoint `POST /api/v1/auth/reset-password` yang menerima email + OTP + password baru. Reset berhasil akan memperbarui password hash dan mencabut seluruh refresh token dalam satu operasi atomik.

#### Scenario: Reset sukses — password berubah, sesi lain dicabut
- **GIVEN** pengguna memiliki OTP `reset_password` yang valid
- **WHEN** pengguna mengirim email + OTP valid + password baru yang memenuhi kebijakan
- **THEN** sistem memperbarui password, mencabut seluruh refresh token, dan membalas 200 OK

#### Scenario: OTP invalid — ditolak
- **GIVEN** pengguna memiliki OTP `reset_password` yang sudah expired atau salah
- **WHEN** pengguna mengirim OTP tersebut
- **THEN** sistem menolak dengan error validasi dan tidak mengubah password

#### Scenario: Password lemah — 422 Validation
- **GIVEN** OTP valid
- **WHEN** password baru tidak memenuhi kebijakan kekuatan
- **THEN** sistem menolak dengan `422 VALIDATION` pada field `new_password`

## Requirement: Change-password (authed) dengan OTP

Sistem SHALL menyediakan dua endpoint untuk mengubah password oleh pengguna terautentikasi:
- `POST /api/v1/auth/change-password/otp` — minta OTP `change_password` ke email terdaftar
- `POST /api/v1/auth/change-password` — verifikasi OTP + set password baru

#### Scenario: Minta OTP change-password — terkirim ke email
- **GIVEN** pengguna terautentikasi dengan JWT valid
- **WHEN** pengguna meminta OTP change-password
- **THEN** sistem kirim OTP `change_password` ke email terdaftar dan balas 200 OK

#### Scenario: Change-password sukses — sesi lain dicabut
- **GIVEN** pengguna terautentikasi dengan OTP `change_password` valid
- **WHEN** pengguna mengirim OTP valid + password baru
- **THEN** sistem memperbarui password, mencabut refresh token sesi LAIN (tidak termasuk sesi saat ini), dan balas 200 OK
