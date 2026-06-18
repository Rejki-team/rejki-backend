## ADDED Requirements

### Requirement: Multi-step write operations dibungkus dalam database transaction

Sistem SHALL membungkus operasi yang melibatkan lebih dari satu pernyataan write pada tabel berbeda dalam sebuah database transaction. Jika salah satu pernyataan gagal, seluruh operasi SHALL di-rollback. Ini berlaku untuk suspend akun (set_status + insert_suspension + revoke_all_refresh_tokens), reset password (revoke_all_refresh_tokens + update_password), change password (revoke_all_refresh_tokens + update_password), dan bump OTP attempts (UPDATE attempts + conditional DELETE).

#### Scenario: Suspend gagal di tengah — rollback otomatis

- **WHEN** operasi suspend menjalankan `set_status` berhasil tetapi `insert_suspension` gagal
- **THEN** transaction di-rollback, status user kembali ke nilai sebelum suspend, tidak ada perubahan parsial

#### Scenario: Reset password gagal saat revoke — rollback otomatis

- **WHEN** `revoke_all_refresh_tokens` gagal dalam operasi reset password
- **THEN** transaction di-rollback, password tidak berubah, token tetap valid — aman untuk retry

#### Scenario: Semua operasi dalam transaction berhasil

- **WHEN** semua pernyataan write dalam transaction berhasil
- **THEN** transaction di-commit, semua perubahan atomik tersimpan

### Requirement: Repository trait menyediakan method untuk transaction-aware operations

Sistem SHALL menyediakan method pada `AuthRepository` trait yang mendukung operasi transactional. Method `suspend_user_transactional`, `update_password_transactional`, dan `bump_otp_attempts_transactional` SHALL menerima `&mut sqlx::Transaction<'_, sqlx::Postgres>` atau equivalent supaya pemanggil dapat mengontrol transaction boundary.

#### Scenario: Transaction dimulai dari application layer

- **WHEN** application service memulai transaction dan memanggil repository method dengan transaction reference
- **THEN** semua operasi menggunakan koneksi transaction yang sama
