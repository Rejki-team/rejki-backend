## MODIFIED Requirements

### Requirement: Penangguhan akun sementara dan permanen

Sistem SHALL menyediakan kapabilitas menangguhkan akun secara sementara maupun permanen. Penangguhan sementara SHALL menyimpan `expires_at`; penangguhan permanen SHALL tidak memiliki `expires_at`. Setiap penangguhan SHALL mencatat `reason` dan `created_by` (identitas admin pemicu). Akun `suspended_temp` SHALL otomatis kembali ke `active` setelah `expires_at` terlewati.

**PENTING:** Operasi suspend SHALL divalidasi terhadap state machine `AccountStatus` — hanya user dengan status `Active` yang dapat ditangguhkan. Operasi suspend (set_status + insert_suspension + revoke_all_refresh_tokens) SHALL dibungkus dalam database transaction untuk menjamin atomicity.

#### Scenario: Penangguhan sementara

- **WHEN** admin menangguhkan akun secara sementara dengan `reason` dan `expires_at`
- **THEN** status akun berubah menjadi `suspended_temp`, dan catatan penangguhan menyimpan `reason`, `expires_at`, serta `created_by` dalam satu transaction atomik

#### Scenario: Penangguhan permanen

- **WHEN** admin menangguhkan akun secara permanen dengan `reason`
- **THEN** status akun berubah menjadi `suspended_permanent` dengan `expires_at` kosong dan catatan menyimpan `reason` serta `created_by` dalam satu transaction atomik

#### Scenario: Penangguhan sementara berakhir

- **WHEN** waktu `expires_at` sebuah akun `suspended_temp` telah terlewati
- **THEN** akun diperlakukan sebagai `active` kembali pada pemeriksaan berikutnya

#### Scenario: Penangguhan ditolak karena status saat ini tidak valid

- **WHEN** admin mencoba menangguhkan akun dengan status selain `Active` (mis. `PendingVerification`, `ProfileIncomplete`, `PendingKyc`, `Rejected`)
- **THEN** sistem menolak dengan error "transisi status tidak sah" dan tidak mengubah status akun

#### Scenario: Penangguhan gagal — rollback otomatis

- **WHEN** salah satu operasi dalam transaction gagal (mis. `insert_suspension` gagal setelah `set_status` berhasil)
- **THEN** seluruh transaction di-rollback, status user tidak berubah, tidak ada catatan parsial

### Requirement: Audit tindakan penangguhan

Sistem SHALL mencatat setiap tindakan penangguhan dengan aktor (`created_by`), sasaran (`user_id`), waktu, dan `reason` sebagai dasar audit.

#### Scenario: Tindakan penangguhan tercatat

- **WHEN** sebuah penangguhan dilakukan
- **THEN** sistem menyimpan catatan audit berisi aktor, sasaran, waktu, dan alasan
