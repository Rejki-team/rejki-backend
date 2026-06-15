## ADDED Requirements

### Requirement: Model data iklan barang bekas gratis

Sistem SHALL merepresentasikan iklan barang bekas gratis dengan atribut: jenis barang (Bekas atau Baru), jumlah barang, lokasi pengambilan, foto barang, dan status. Sistem SHALL TIDAK menyertakan harga sebagai atribut iklan barang bekas gratis. Jumlah barang SHALL bernilai minimal satu.

#### Scenario: Pengguna membuat iklan barang bekas gratis

- **WHEN** pengguna membuat iklan dengan jenis barang, jumlah, lokasi pengambilan, dan foto
- **THEN** sistem menyimpan iklan tanpa atribut harga

#### Scenario: Jumlah tidak valid

- **WHEN** pengguna membuat iklan dengan jumlah kurang dari satu
- **THEN** sistem menolak permintaan dengan galat validasi

#### Scenario: Jenis barang di luar pilihan

- **WHEN** pengguna mengirim jenis barang selain Bekas atau Baru
- **THEN** sistem menolak permintaan dengan galat validasi

### Requirement: Status ketersediaan barang

Sistem SHALL melacak status ketersediaan iklan barang bekas gratis dengan nilai Tersedia atau Sudah Diambil. Sistem SHALL menyediakan transisi untuk menandai barang sebagai Sudah Diambil. Status ketersediaan SHALL terpisah dari status moderasi (mis. tersuspensi).

#### Scenario: Menandai barang sudah diambil

- **WHEN** pemilik menandai barang sebagai sudah diambil
- **THEN** status ketersediaan iklan menjadi Sudah Diambil

#### Scenario: Status moderasi terpisah dari ketersediaan

- **WHEN** iklan tersuspensi oleh moderasi
- **THEN** status moderasi mencerminkan tersuspensi tanpa mengubah status ketersediaan

### Requirement: Surfacing kolom barang gratis pada listing admin dan CSV

Sistem SHALL menyertakan jenis barang, jumlah, lokasi pengambilan, dan status ketersediaan pada daftar admin dan pada ekspor CSV iklan barang bekas gratis, sesuai pencarian dan filter yang sedang aktif.

#### Scenario: Admin melihat daftar dengan kolom gratis

- **WHEN** admin membuka daftar iklan barang bekas gratis
- **THEN** setiap baris menampilkan jenis barang, jumlah, lokasi pengambilan, dan status ketersediaan

#### Scenario: Ekspor CSV memuat kolom gratis

- **WHEN** admin mengekspor CSV iklan barang bekas gratis
- **THEN** berkas CSV memuat kolom jenis barang, jumlah, lokasi pengambilan, dan status ketersediaan
