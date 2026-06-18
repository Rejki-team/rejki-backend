## ADDED Requirements

### Requirement: Pengguna dapat melaporkan iklan atau pengguna lain

Sistem SHALL mengizinkan pengguna terautentikasi membuat laporan/aduan terhadap iklan atau pengguna lain dengan menyertakan keterangan dan bukti foto opsional. Laporan yang dibuat SHALL berstatus `pending` hingga ditinjau admin.

#### Scenario: Pengguna melaporkan iklan
- **WHEN** pengguna melaporkan iklan dengan keterangan dan bukti
- **THEN** sistem menciptakan aduan berstatus `pending` dengan jenis target `iklan`

#### Scenario: Pengguna melaporkan pengguna
- **WHEN** pengguna melaporkan pengguna lain dengan keterangan
- **THEN** sistem menciptakan aduan berstatus `pending` dengan jenis target `user`

### Requirement: Antarmuka ReportClient untuk domain lain

Sistem SHALL menyediakan antarmuka `ReportClient` yang dapat dipanggil in-process oleh domain lain untuk membuat laporan dan memeriksa status laporan.

#### Scenario: Domain lain membuat laporan
- **WHEN** domain lain memanggil operasi pembuatan laporan dengan data yang valid
- **THEN** sistem menciptakan laporan dan mengembalikan pengenalnya
