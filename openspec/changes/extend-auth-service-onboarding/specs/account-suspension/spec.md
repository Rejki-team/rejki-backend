## ADDED Requirements

### Requirement: Penangguhan akun sementara dan permanen

Sistem SHALL menyediakan kapabilitas menangguhkan akun secara sementara maupun permanen. Penangguhan sementara SHALL menyimpan `expires_at`; penangguhan permanen SHALL tidak memiliki `expires_at`. Setiap penangguhan SHALL mencatat `reason` dan `created_by` (identitas admin pemicu). Akun `suspended_temp` SHALL otomatis kembali ke `active` setelah `expires_at` terlewati.

#### Scenario: Penangguhan sementara
- **WHEN** admin menangguhkan akun secara sementara dengan `reason` dan `expires_at`
- **THEN** status akun berubah menjadi `suspended_temp`, dan catatan penangguhan menyimpan `reason`, `expires_at`, serta `created_by`

#### Scenario: Penangguhan permanen
- **WHEN** admin menangguhkan akun secara permanen dengan `reason`
- **THEN** status akun berubah menjadi `suspended_permanent` dengan `expires_at` kosong dan catatan menyimpan `reason` serta `created_by`

#### Scenario: Penangguhan sementara berakhir
- **WHEN** waktu `expires_at` sebuah akun `suspended_temp` telah terlewati
- **THEN** akun diperlakukan sebagai `active` kembali pada pemeriksaan berikutnya

### Requirement: Penegakan penangguhan saat login dan validasi token

Sistem SHALL menolak login akun yang sedang ditangguhkan dengan galat yang jelas. Sistem SHALL menolak permintaan terotorisasi dari akun yang sedang ditangguhkan meskipun token masih dalam masa berlaku.

#### Scenario: Login akun yang ditangguhkan
- **WHEN** akun `suspended_temp` (belum kedaluwarsa) atau `suspended_permanent` mencoba login
- **THEN** sistem menolak dengan galat yang menyatakan akun ditangguhkan dan tidak menerbitkan token

#### Scenario: Token berlaku tetapi akun ditangguhkan
- **WHEN** akun ditangguhkan setelah token diterbitkan dan token masih berlaku
- **THEN** sistem menolak permintaan terotorisasi berdasarkan pemeriksaan kesegaran status

### Requirement: Audit tindakan penangguhan

Sistem SHALL mencatat setiap tindakan penangguhan dengan aktor (`created_by`), sasaran (`user_id`), waktu, dan `reason` sebagai dasar audit.

#### Scenario: Tindakan penangguhan tercatat
- **WHEN** sebuah penangguhan dilakukan
- **THEN** sistem menyimpan catatan audit berisi aktor, sasaran, waktu, dan alasan
