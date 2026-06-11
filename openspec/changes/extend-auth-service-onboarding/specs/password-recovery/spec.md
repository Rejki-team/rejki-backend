## ADDED Requirements

### Requirement: Lupa password dengan otorisasi OTP

Sistem SHALL menyediakan permintaan lupa password yang mengirim OTP dengan purpose `reset_password` ke email terdaftar. Demi anti-enumeration, sistem SHALL selalu mengembalikan respons sukses generik terlepas dari apakah email terdaftar.

#### Scenario: Permintaan lupa password untuk email terdaftar
- **WHEN** pengguna meminta lupa password untuk email yang terdaftar
- **THEN** sistem memicu pengiriman OTP `reset_password` ke email tersebut dan membalas respons sukses generik

#### Scenario: Permintaan lupa password untuk email tidak terdaftar
- **WHEN** pengguna meminta lupa password untuk email yang tidak terdaftar
- **THEN** sistem membalas respons sukses generik yang identik tanpa mengirim OTP dan tanpa membocorkan ketiadaan akun

### Requirement: Reset password mencabut seluruh sesi

Sistem SHALL mengizinkan penetapan password baru bila OTP `reset_password` valid dan password baru memenuhi kebijakan. Setelah reset berhasil, sistem SHALL mencabut seluruh refresh token milik pengguna.

#### Scenario: Reset password berhasil
- **WHEN** pengguna mengirim OTP `reset_password` yang valid beserta password baru yang memenuhi kebijakan
- **THEN** sistem memperbarui password (sebagai hash), mencabut seluruh refresh token pengguna, dan membalas sukses

#### Scenario: OTP reset tidak valid atau kedaluwarsa
- **WHEN** pengguna mengirim OTP `reset_password` yang salah atau sudah kedaluwarsa
- **THEN** sistem menolak dengan galat otorisasi dan tidak mengubah password

#### Scenario: Password baru tidak memenuhi kebijakan
- **WHEN** OTP valid tetapi password baru tidak memenuhi kebijakan kekuatan
- **THEN** sistem menolak dengan `422` dan kode `VALIDATION` pada field `password`

### Requirement: Ubah password oleh pengguna terautentikasi dengan OTP

Sistem SHALL mengizinkan pengguna terautentikasi mengubah password setelah otorisasi OTP dengan purpose `change_password`. Setelah perubahan berhasil, sistem SHALL mencabut refresh token lain milik pengguna.

#### Scenario: Ubah password berhasil
- **WHEN** pengguna terautentikasi mengirim OTP `change_password` yang valid dan password baru yang memenuhi kebijakan
- **THEN** sistem memperbarui password (sebagai hash) dan mencabut refresh token lain milik pengguna

#### Scenario: Ubah password tanpa otorisasi OTP yang valid
- **WHEN** pengguna terautentikasi mengirim permintaan ubah password dengan OTP `change_password` yang salah atau kedaluwarsa
- **THEN** sistem menolak dengan galat otorisasi dan tidak mengubah password
