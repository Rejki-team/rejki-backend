## ADDED Requirements

### Requirement: Akses fitur dibatasi oleh status akun aktif

Sistem SHALL hanya mengizinkan akun berstatus `active` untuk mengakses endpoint fitur (mis. iklan, chat). Permintaan ke endpoint fitur dari akun berstatus selain `active` SHALL ditolak dengan `403` dan kode mesin `ACCOUNT_NOT_ACTIVE`.

#### Scenario: Akun aktif mengakses fitur
- **WHEN** akun berstatus `active` mengakses endpoint fitur dengan token valid
- **THEN** sistem mengizinkan permintaan diproses

#### Scenario: Akun belum lengkap mengakses fitur
- **WHEN** akun berstatus `profile_incomplete` atau `pending_kyc` mengakses endpoint fitur
- **THEN** sistem menolak dengan `403` dan kode `ACCOUNT_NOT_ACTIVE`

#### Scenario: Akun ditangguhkan mengakses fitur
- **WHEN** akun berstatus `suspended_temp` atau `suspended_permanent` mengakses endpoint fitur
- **THEN** sistem menolak dengan `403` dan kode `ACCOUNT_NOT_ACTIVE`

### Requirement: Autentikasi terpasang sebelum gating pada endpoint fitur

Sistem SHALL memastikan endpoint fitur yang membutuhkan identitas pengguna melewati autentikasi (penyuntikan identitas dari token) sebelum lapisan gating status diterapkan. Endpoint publik yang tidak membutuhkan login (mis. melihat iklan yang diterbitkan) SHALL TIDAK dikenai gating status.

#### Scenario: Endpoint fitur memerlukan autentikasi
- **WHEN** permintaan tanpa token valid menuju endpoint fitur yang membutuhkan identitas
- **THEN** sistem menolak dengan `401` sebelum mengevaluasi status akun

#### Scenario: Endpoint publik tidak dikenai gating status
- **WHEN** permintaan menuju endpoint publik yang memang tidak membutuhkan login
- **THEN** sistem tidak menolaknya berdasarkan status akun

### Requirement: Penegakan status menggabungkan klaim JWT dan pemeriksaan kesegaran

Sistem SHALL menyertakan status akun di dalam klaim JWT untuk pemeriksaan cepat. Untuk peristiwa sensitif (penangguhan, perubahan status), sistem SHALL menjamin kesegaran melalui pemeriksaan cepat ke penyimpanan status/daftar pencabutan (Redis) sehingga token yang masih berlaku tidak dapat menembus pembatasan setelah status berubah. Untuk token yang diterbitkan sebelum field status ada, sistem SHALL melakukan pemeriksaan kesegaran sebagai cadangan.

#### Scenario: Token masih berlaku tetapi akun baru ditangguhkan
- **WHEN** sebuah token JWT masih dalam masa berlaku namun status akun telah berubah menjadi `suspended_temp` sejak token diterbitkan
- **THEN** sistem menolak permintaan ke endpoint fitur berdasarkan hasil pemeriksaan kesegaran, bukan berdasarkan klaim usang di token

#### Scenario: Akun aktif tanpa perubahan status
- **WHEN** akun `active` mengakses fitur dan tidak ada catatan perubahan status pada penyimpanan kesegaran
- **THEN** sistem mengizinkan permintaan berdasarkan klaim JWT tanpa beban kueri tambahan ke basis data utama
