## ADDED Requirements

### Requirement: Suspend iklan wajib menyertakan alasan dan bukti

Sistem SHALL mengizinkan admin men-suspend satu, sebagian, atau sekaligus banyak iklan. Setiap suspend SHALL mewajibkan `reason` dan **bukti** (berkas gambar atau PDF, maksimal 5MB, satu berkas) yang diunggah via presigned URL kategori bukti suspensi. Suspend SHALL menyimpan riwayat suspensi (`reason`, `evidence_object_key`, `created_by`).

#### Scenario: Suspend tanpa bukti ditolak
- **WHEN** admin men-suspend iklan tanpa menyertakan bukti
- **THEN** sistem menolak permintaan dengan galat validasi

#### Scenario: Suspend dengan alasan dan bukti berhasil
- **WHEN** admin men-suspend iklan dengan `reason` dan bukti yang valid
- **THEN** iklan berpindah ke status moderasi tersuspensi dan riwayat suspensi tersimpan

### Requirement: Suspend sementara versus permanen

Sistem SHALL mendukung suspend sementara (dengan `expires_at`) dan suspend permanen. Suspend sementara SHALL menyetel `moderation_status=suspended_temp` dengan waktu kedaluwarsa; suspend permanen SHALL menyetel `suspended_permanent` tanpa kedaluwarsa.

#### Scenario: Suspend sementara menetapkan kedaluwarsa
- **WHEN** admin memilih suspend sementara
- **THEN** iklan menjadi `suspended_temp` dengan `expires_at` terisi

#### Scenario: Suspend permanen tanpa kedaluwarsa
- **WHEN** admin memilih suspend permanen
- **THEN** iklan menjadi `suspended_permanent` tanpa `expires_at`

### Requirement: Notifikasi otomatis ke pemilik iklan terdampak

Sistem SHALL mengirim notifikasi otomatis (email dan in-app) ke pemilik iklan yang terdampak setiap kali suspend dilakukan.

#### Scenario: Pemilik menerima notifikasi suspend
- **WHEN** sebuah iklan di-suspend oleh admin
- **THEN** sistem mengirim notifikasi email dan in-app ke pemilik iklan tersebut

#### Scenario: Suspend massal memberi tahu setiap pemilik terdampak
- **WHEN** admin men-suspend beberapa iklan sekaligus
- **THEN** sistem memberi tahu setiap pemilik iklan yang terdampak
