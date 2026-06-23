## ADDED Requirements

### Requirement: Foto iklan tersedia pada respons untuk ditampilkan sebagai popup

Sistem SHALL menyertakan referensi foto iklan (foto pekerjaan/barang) pada respons sehingga klien dashboard dapat menampilkan setiap foto secara jelas pada popup. Foto SHALL disimpan dan diakses melalui mekanisme object storage yang konsisten dengan media lain.

#### Scenario: Respons memuat foto bila tersedia
- **WHEN** sebuah iklan memiliki foto terunggah
- **THEN** respons memuat referensi foto tersebut sehingga dapat ditampilkan satu per satu

#### Scenario: Iklan tanpa foto
- **WHEN** sebuah iklan tidak memiliki foto
- **THEN** respons memuat daftar foto kosong tanpa galat
