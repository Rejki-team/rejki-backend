## ADDED Requirements

### Requirement: Iklan menyimpan region_id terstruktur

Keempat entitas iklan (`pekerjaan`, `pekerja`, `barang-bekas`, `pelatihan`) SHALL memiliki kolom `region_id TEXT` yang nullable dan menyimpan kode wilayah dari `region-service`. Kolom `lokasi` free-text TETAP dipertahankan. `region_id` bersifat opsional — iklan tanpa `region_id` tetap valid.

#### Scenario: Create iklan dengan region_id valid

- **WHEN** user membuat iklan dengan `region_id` = "32.73" (Kota Bandung)
- **THEN** system memvalidasi `region_id` via `RegionClient::get_region("32.73")` dan menyimpan iklan dengan `region_id` = "32.73"

#### Scenario: Create iklan tanpa region_id

- **WHEN** user membuat iklan tanpa mengisi `region_id`
- **THEN** system menyimpan iklan dengan `region_id = NULL` dan TIDAK memanggil `RegionClient`

#### Scenario: Read iklan mengembalikan region_id

- **WHEN** client membaca iklan yang memiliki `region_id`
- **THEN** response DTO menyertakan field `region_id` dengan nilai yang tersimpan

### Requirement: Validasi region_id via RegionClient pada create

Saat iklan dibuat, jika `region_id` diisi, system SHALL memanggil `RegionClient::get_region(region_id)`. Jika region tidak ditemukan (`NotFound`), system SHALL menolak dengan 422. Jika region-service tidak tersedia (`Unavailable`), system SHALL menerima create dengan warning log (fail-open).

#### Scenario: region_id tidak ditemukan di region-service

- **WHEN** user membuat iklan dengan `region_id` = "99.99" (region tidak ada)
- **THEN** system mengembalikan error 422 dengan pesan "region_id tidak ditemukan"

#### Scenario: region-service tidak tersedia saat create

- **WHEN** user membuat iklan dengan `region_id` valid tapi region-service sedang down
- **THEN** system tetap menyimpan iklan (fail-open) dan mencatat WARN log

### Requirement: Validasi region_id pada update pelatihan

Service `iklan-pelatihan` memiliki endpoint update (`update_pelatihan`). Saat update, jika `region_id` diisi, system SHALL memvalidasi dengan cara yang sama seperti create.

#### Scenario: Update pelatihan dengan region_id baru yang valid

- **WHEN** pemilik mengupdate pelatihan dengan `region_id` = "32.73"
- **THEN** system memvalidasi region dan menyimpan perubahan

#### Scenario: Update pelatihan dengan region_id tidak valid

- **WHEN** pemilik mengupdate pelatihan dengan `region_id` = "99.99"
- **THEN** system mengembalikan error 422 dengan pesan "region_id tidak ditemukan"
