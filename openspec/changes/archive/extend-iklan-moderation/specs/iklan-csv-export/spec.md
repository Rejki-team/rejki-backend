## ADDED Requirements

### Requirement: Ekspor daftar iklan ke CSV mengikuti filter aktif

Sistem SHALL menyediakan endpoint ekspor CSV per vertikal (diproteksi otorisasi admin) yang menghasilkan unduhan `text/csv` berisi daftar iklan sesuai pencarian/filter yang sedang aktif. CSV SHALL TIDAK memuat data sensitif.

#### Scenario: Admin mengekspor daftar terfilter
- **WHEN** admin meminta ekspor CSV dengan filter/pencarian tertentu
- **THEN** sistem mengembalikan berkas CSV berisi header kolom dan baris iklan yang sesuai filter

#### Scenario: Ekspor tidak memuat data sensitif
- **WHEN** admin mengekspor daftar iklan
- **THEN** CSV tidak memuat data sensitif (mis. NIK atau dokumen identitas)
