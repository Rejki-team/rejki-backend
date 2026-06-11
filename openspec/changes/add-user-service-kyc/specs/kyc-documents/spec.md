## ADDED Requirements

### Requirement: Unggah dokumen KTP dan swafoto via presigned URL

Sistem SHALL menyediakan alur unggah dua langkah untuk dokumen KYC (foto KTP dan swafoto-dengan-KTP): klien meminta izin unggah (menyebut jenis dokumen, tipe MIME, ukuran), sistem memvalidasi lalu mengembalikan presigned URL + object key; klien mengunggah langsung ke object storage. Sistem SHALL memvalidasi tipe MIME dan ukuran, membuat nama objek sendiri, dan memverifikasi magic bytes berkas saat commit.

#### Scenario: Permintaan unggah dokumen valid
- **WHEN** pengguna meminta unggah dokumen KYC dengan jenis dikenal, tipe MIME diizinkan, dan ukuran dalam batas
- **THEN** sistem mengembalikan presigned URL dan object key yang dibuat server

#### Scenario: Berkas tidak sesuai tipe yang diklaim
- **WHEN** berkas yang diunggah tidak lolos verifikasi magic bytes terhadap tipe MIME yang diizinkan
- **THEN** sistem menolak penyimpanan referensi dokumen tersebut

#### Scenario: Tipe atau ukuran tidak diizinkan
- **WHEN** permintaan unggah menyebut tipe MIME tidak diizinkan atau ukuran melebihi batas
- **THEN** sistem menolak dengan `422` dan kode `VALIDATION`

### Requirement: Dokumen KYC tidak dapat diakses publik

Sistem SHALL menyimpan object key dokumen (bukan URL publik permanen). Akses baca dokumen SHALL terbatas pada pemilik dan admin peninjau, idealnya melalui presigned URL baca berumur pendek. Dokumen SHALL TIDAK dapat diakses melalui URL publik.

#### Scenario: Akses dokumen oleh non-pemilik dan non-admin
- **WHEN** pihak yang bukan pemilik dan bukan admin peninjau mencoba mengakses dokumen KYC
- **THEN** sistem tidak memberikan akses

#### Scenario: Dokumen tidak terekspos sebagai URL publik
- **WHEN** dokumen KYC disimpan
- **THEN** sistem menyimpan object key dan tidak mengekspos URL publik permanen ke dokumen tersebut

### Requirement: Retensi dokumen sesuai prinsip pembatasan penyimpanan

Sistem SHALL menyimpan dokumen KYC selama akun aktif sebagai bagian dari tujuan pemrosesan, dan SHALL menyediakan mekanisme pemusnahan dokumen setelah akun ditutup (ditambah periode dispute) sesuai prinsip pembatasan penyimpanan UU PDP.

#### Scenario: Dokumen dimusnahkan setelah akun ditutup
- **WHEN** sebuah akun ditutup dan periode dispute telah terlewati
- **THEN** sistem memusnahkan dokumen KYC terkait
