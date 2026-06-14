## ADDED Requirements

### Requirement: State moderasi dan soft-delete pada iklan

Sistem SHALL menyimpan `moderation_status` (`active`/`suspended_temp`/`suspended_permanent`) dan penanda penghapusan lunak (`deleted_at`) pada keempat vertikal iklan. State moderasi SHALL orthogonal terhadap flag fungsional pemilik (`is_active`/`is_sold`). Iklan baru SHALL default `moderation_status=active`.

#### Scenario: Iklan baru aktif secara moderasi
- **WHEN** pengguna membuat iklan baru
- **THEN** iklan memiliki `moderation_status=active` dan tidak ber-`deleted_at`

### Requirement: List publik mengecualikan iklan tersuspensi atau terhapus

Sistem SHALL menampilkan pada daftar publik hanya iklan dengan `moderation_status=active` dan tanpa `deleted_at`. Iklan yang di-suspend atau di-soft-delete oleh admin SHALL TIDAK muncul pada daftar publik.

#### Scenario: Iklan tersuspensi disembunyikan dari publik
- **WHEN** sebuah iklan berada pada `moderation_status=suspended_temp` atau `suspended_permanent`
- **THEN** iklan tersebut tidak muncul pada daftar publik

#### Scenario: Iklan terhapus lunak disembunyikan dari publik
- **WHEN** sebuah iklan memiliki `deleted_at` terisi
- **THEN** iklan tersebut tidak muncul pada daftar publik
