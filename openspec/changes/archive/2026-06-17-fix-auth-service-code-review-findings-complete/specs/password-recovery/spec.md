## MODIFIED Requirements

### Requirement: Reset password mencabut seluruh sesi

Sistem SHALL mengizinkan penetapan password baru bila OTP `reset_password` valid dan password baru memenuhi kebijakan. Setelah reset berhasil, sistem SHALL mencabut seluruh refresh token milik pengguna. **PENTING:** Operasi reset password (revoke_all_refresh_tokens + update_password) SHALL dibungkus dalam database transaction — token dicabut DULU, baru password diperbarui. Jika salah satu gagal, seluruh operasi di-rollback.

#### Scenario: Reset password berhasil

- **WHEN** pengguna mengirim OTP `reset_password` yang valid beserta password baru yang memenuhi kebijakan
- **THEN** sistem mencabut seluruh refresh token pengguna terlebih dahulu, kemudian memperbarui password (sebagai hash), keduanya dalam satu transaction atomik

#### Scenario: OTP reset tidak valid atau kedaluwarsa

- **WHEN** pengguna mengirim OTP `reset_password` yang salah atau sudah kedaluwarsa
- **THEN** sistem menolak dengan galat otorisasi dan tidak mengubah password

#### Scenario: Password baru tidak memenuhi kebijakan

- **WHEN** OTP valid tetapi password baru tidak memenuhi kebijakan kekuatan
- **THEN** sistem menolak dengan `422` dan kode `VALIDATION` pada field `password`

#### Scenario: Reset password gagal saat revoke token — rollback

- **WHEN** `revoke_all_refresh_tokens` gagal (mis. transient DB error) dalam transaction reset password
- **THEN** entire transaction di-rollback, password tidak berubah, token tetap valid, user dapat retry

### Requirement: Ubah password oleh pengguna terautentikasi dengan OTP

Sistem SHALL mengizinkan pengguna terautentikasi mengubah password setelah otorisasi OTP dengan purpose `change_password`. Setelah perubahan berhasil, sistem SHALL mencabut refresh token lain milik pengguna. **PENTING:** Operasi change password (revoke_all_refresh_tokens + update_password) SHALL dibungkus dalam database transaction — token dicabut DULU, baru password diperbarui.

#### Scenario: Ubah password berhasil

- **WHEN** pengguna terautentikasi mengirim OTP `change_password` yang valid dan password baru yang memenuhi kebijakan
- **THEN** sistem mencabut refresh token lain milik pengguna terlebih dahulu, kemudian memperbarui password (sebagai hash), keduanya dalam satu transaction atomik

#### Scenario: Ubah password tanpa otorisasi OTP yang valid

- **WHEN** pengguna terautentikasi mengirim permintaan ubah password dengan OTP `change_password` yang salah atau kedaluwarsa
- **THEN** sistem menolak dengan galat otorisasi dan tidak mengubah password
