## ADDED Requirements

### Requirement: Akses dokumen KYC pengguna oleh admin

Sistem SHALL menyediakan endpoint admin yang mengembalikan URL akses sementara untuk dokumen KYC (Foto KTP atau Foto Selfie dengan KTP) milik pengajuan tertentu. Endpoint SHALL diproteksi sehingga hanya pembawa token ber-role admin yang dapat mengaksesnya. Sistem SHALL membatasi jenis dokumen yang dapat diminta pada KTP dan Selfie.

#### Scenario: Admin membuka Foto KTP

- **WHEN** admin meminta dokumen KTP untuk satu pengajuan KYC
- **THEN** sistem mengembalikan URL akses sementara untuk dokumen tersebut

#### Scenario: Jenis dokumen tidak dikenal

- **WHEN** admin meminta dokumen dengan jenis selain KTP atau Selfie
- **THEN** sistem menolak permintaan dengan galat validasi

### Requirement: Pencatatan audit setiap akses dokumen oleh admin

Sistem SHALL mencatat setiap penerbitan URL akses dokumen oleh admin ke dalam jejak audit, mencakup identitas admin sebagai aktor, objek dokumen yang diakses, dan waktu akses. Data dokumen asli SHALL TIDAK pernah ikut tercatat dalam log.

#### Scenario: Akses dokumen tercatat

- **WHEN** admin meminta URL dokumen KTP atau Selfie
- **THEN** sistem mencatat akses dengan aktor admin, objek dokumen, dan waktu, tanpa mencatat isi dokumen

#### Scenario: Dokumen sudah dimusnahkan

- **WHEN** admin meminta dokumen pada pengajuan yang dokumennya telah dihapus
- **THEN** sistem mengembalikan keadaan tidak-tersedia tanpa menerbitkan URL
