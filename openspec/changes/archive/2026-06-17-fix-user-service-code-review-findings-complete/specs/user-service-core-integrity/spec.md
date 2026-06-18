## MODIFIED Requirements

### Requirement: NIK immutable di level aplikasi DAN database

Sistem SHALL menjaga NIK tetap immutable setelah diset. Guard di level aplikasi (`find_by_auth_id`
single fetch) diverifikasi oleh guard di level database (`UPDATE ... WHERE nik_encrypted IS NULL`).
Sistem SHALL mengembalikan error bila NIK sudah ada (idempoten).

**CLAUDEMD Ref:** §4.5 — "Zero Race Condition: operasi gabungan harus atomik — bungkus dalam
transaction (`pool.begin()` … `commit()`), atau pola DELETE-RETURNING"

#### Scenario: NIK tidak dapat di-overwrite secara concurrent
- **WHEN** dua request `submit_kyc` konkuren untuk user yang sama keduanya lolos pengecekan aplikasi
- **THEN** hanya satu yang berhasil menulis NIK; yang kedua mendapat error karena `AND nik_encrypted IS NULL` tidak terpenuhi

#### Scenario: NIK immutable melalui self-service update
- **WHEN** pengguna mencoba mengubah NIK via `PATCH /api/v1/users/me`
- **THEN** sistem menolak perubahan karena field NIK tidak termasuk dalam `UpdateProfileInput`

---

### Requirement: Submit KYC dalam transaction boundary

Sistem SHALL membungkus operasi tulis `find_by_auth_id` + `update_profile` + `create_submission`
dalam satu database transaction. Jika `create_submission` gagal, seluruh perubahan (termasuk
penyimpanan NIK) di-rollback. Operasi `set_account_status` (AuthClient) dijalankan setelah commit
sebagai best-effort.

**CLAUDEMD Ref:** §4.5 — "pastikan setiap `begin()` berakhir `commit`/`rollback`"

#### Scenario: Partial failure tidak meninggalkan state tidak konsisten
- **WHEN** `update_profile` sukses tapi `create_submission` gagal (mis. constraint violation, connection drop)
- **THEN** transaction di-rollback, NIK tidak disimpan, user dapat retry submit KYC tanpa error "NIK tidak dapat diubah"

#### Scenario: Transaction commit sukses, AuthClient gagal
- **WHEN** transaction commit sukses tapi `set_account_status(PendingKyc)` gagal (AuthClient timeout)
- **THEN** submission KYC sudah tercatat dengan benar; status akun tetap `profile_incomplete`
  (trade-off diterima — lebih baik daripada NIK tersimpan tanpa submission)

---
