## ADDED Requirements

### Requirement: Unggah dan ganti foto profil via presigned URL

Sistem SHALL menyediakan alur unggah avatar dua langkah: klien meminta izin unggah (menyebut tipe konten & ukuran), sistem memvalidasi lalu mengembalikan presigned URL beserta object key; klien mengunggah langsung ke object storage. Sistem SHALL memvalidasi tipe MIME dan batas ukuran, dan SHALL membuat nama objek sendiri (bukan dari nama berkas klien). Avatar lama SHALL dapat diganti.

#### Scenario: Permintaan unggah avatar yang valid
- **WHEN** pengguna meminta unggah avatar dengan tipe MIME gambar yang diizinkan dan ukuran dalam batas
- **THEN** sistem mengembalikan presigned URL dan object key yang dibuat server untuk dipakai klien mengunggah

#### Scenario: Tipe atau ukuran tidak diizinkan
- **WHEN** pengguna meminta unggah avatar dengan tipe MIME tidak diizinkan atau ukuran melebihi batas
- **THEN** sistem menolak dengan `422` dan kode `VALIDATION`

#### Scenario: Mengganti avatar
- **WHEN** pengguna menyelesaikan unggah avatar baru
- **THEN** sistem memperbarui referensi avatar profil ke objek baru
