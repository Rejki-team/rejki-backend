## ADDED Requirements

### Requirement: Pengiriman KYC lengkap memicu transisi ke antrian verifikasi

Sistem SHALL membuat submission KYC berstatus `pending` ketika data diri lengkap dan dokumen wajib telah terkirim. Sistem SHALL memicu transisi status akun `profile_incomplete → pending_kyc` melalui `AuthClient.set_account_status`.

#### Scenario: Submission KYC dibuat
- **WHEN** pengguna `profile_incomplete` menyelesaikan pengiriman data diri lengkap dan dokumen wajib
- **THEN** sistem membuat submission berstatus `pending` dan memicu transisi akun ke `pending_kyc` via `AuthClient`

#### Scenario: Submission diblokir bila data/dokumen belum lengkap
- **WHEN** pengguna mencoba mengirim KYC tanpa seluruh data diri atau dokumen wajib
- **THEN** sistem tidak membuat submission dan mengembalikan galat validasi

### Requirement: Review admin menyetujui atau menolak submission

Sistem SHALL menyediakan operasi review (kelak diproteksi RBAC admin) untuk menyetujui atau menolak submission `pending`. Approve SHALL memicu transisi akun `pending_kyc → active` via `AuthClient`. Reject SHALL memicu transisi `pending_kyc → rejected` dan menyimpan alasan penolakan.

#### Scenario: Admin menyetujui submission
- **WHEN** admin menyetujui submission `pending`
- **THEN** submission menjadi `approved` dan status akun berubah menjadi `active` via `AuthClient`

#### Scenario: Admin menolak submission dengan alasan
- **WHEN** admin menolak submission `pending` dengan alasan
- **THEN** submission menjadi `rejected`, alasan tersimpan, dan status akun berubah menjadi `rejected` via `AuthClient`

### Requirement: Pengajuan ulang setelah penolakan dengan cooldown

Sistem SHALL mengizinkan akun `rejected` mengajukan ulang KYC hanya setelah masa cooldown 3 hari kerja terlewati sejak penolakan. Pengajuan ulang yang sah SHALL memicu transisi kembali ke `pending_kyc`.

#### Scenario: Pengajuan ulang sebelum cooldown berakhir
- **WHEN** akun `rejected` mencoba mengajukan ulang sebelum 3 hari kerja terlewati
- **THEN** sistem menolak pengajuan ulang dan memberi tahu kapan dapat mengajukan kembali

#### Scenario: Pengajuan ulang setelah cooldown
- **WHEN** akun `rejected` mengajukan ulang setelah 3 hari kerja terlewati dengan data yang diperbaiki
- **THEN** sistem membuat submission baru `pending` dan memicu transisi akun ke `pending_kyc`
