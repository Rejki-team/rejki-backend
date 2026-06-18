## ADDED Requirements

### Requirement: Tujuh status lifecycle pelatihan

Sistem SHALL melacak pelatihan melalui tujuh status lifecycle: `verifikasi_tertunda`, `verifikasi_dalam_proses`, `verifikasi_ditolak`, `verifikasi_diterima`, `pelatihan_belum_dimulai`, `pelatihan_berjalan`, `pelatihan_selesai`. Sistem SHALL membedakan pelatihan berdasarkan pembuatnya (`created_by_role`: `admin` atau `user`) untuk menentukan alur awal.

#### Scenario: Pelatihan oleh admin langsung disetujui (auto-approve)
- **WHEN** admin membuat pelatihan baru
- **THEN** pelatihan berstatus `verifikasi_diterima` dan memiliki `created_by_role=admin`

#### Scenario: Pelatihan oleh user membutuhkan verifikasi
- **WHEN** pengguna reguler mengajukan pelatihan baru
- **THEN** pelatihan berstatus `verifikasi_tertunda` dan memiliki `created_by_role=user`

### Requirement: Review admin atas pengajuan pelatihan pengguna

Sistem SHALL menyediakan operasi review untuk pelatihan yang diajukan pengguna reguler. Admin SHALL dapat menyetujui (memindahkan ke `verifikasi_diterima`) atau menolak (memindahkan ke `verifikasi_ditolak` dengan alasan wajib). Tombol Tolak dan Terima SHALL terkunci setelah verifikasi dilakukan.

#### Scenario: Admin menolak dengan alasan wajib
- **WHEN** admin menolak pengajuan pelatihan tanpa menyertakan alasan
- **THEN** sistem menolak dengan galat validasi

#### Scenario: Verifikasi tidak dapat diulang
- **WHEN** admin mencoba menyetujui atau menolak pelatihan yang sudah diverifikasi
- **THEN** sistem menolak operasi tersebut

### Requirement: Admin dapat menyunting hanya pelatihan miliknya

Sistem SHALL mengizinkan admin menyunting pelatihan yang ia buat sendiri (`created_by_role=admin`). Sistem SHALL TIDAK mengizinkan admin menyunting pelatihan yang dibuat pengguna reguler.

#### Scenario: Admin menyunting pelatihan miliknya
- **WHEN** admin menyunting pelatihan dengan `created_by_role=admin`
- **THEN** sistem mengizinkan dan menyimpan perubahan

#### Scenario: Admin ditolak menyunting pelatihan pengguna
- **WHEN** admin mencoba menyunting pelatihan dengan `created_by_role=user`
- **THEN** sistem menolak operasi penyuntingan (admin hanya dapat menyetujui atau menolak)
