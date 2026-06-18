## ADDED Requirements

### Requirement: Registrasi kategori storage terpusat

Sistem SHALL mendaftarkan seluruh kategori storage di satu lokasi terpusat (`StorageInProcessClient::request_upload`), masing-masing dengan batas ukuran maksimum dan whitelist MIME yang diizinkan. Setiap kategori SHALL didokumentasikan dengan referensi ke change OpenSpec yang menambahkannya.

#### Scenario: Kategori avatar
- **WHEN** klien meminta upload dengan kategori `avatar`
- **THEN** sistem mengizinkan MIME `image/jpeg` dan `image/png` dengan ukuran maksimal 5MB

#### Scenario: Kategori ktp dan selfie
- **WHEN** klien meminta upload dengan kategori `ktp` atau `selfie`
- **THEN** sistem mengizinkan MIME `image/jpeg` dan `image/png` dengan ukuran maksimal 10MB

#### Scenario: Kategori suspension-evidence
- **WHEN** klien meminta upload dengan kategori `suspension-evidence`
- **THEN** sistem mengizinkan MIME `image/jpeg`, `image/png`, dan `application/pdf` dengan ukuran maksimal 5MB

#### Scenario: Kategori iklan-suspension-evidence
- **WHEN** klien meminta upload dengan kategori `iklan-suspension-evidence`
- **THEN** sistem mengizinkan MIME `image/jpeg`, `image/png`, dan `application/pdf` dengan ukuran maksimal 5MB

#### Scenario: Kategori training-transfer-evidence
- **WHEN** klien meminta upload dengan kategori `training-transfer-evidence`
- **THEN** sistem mengizinkan MIME `image/jpeg`, `image/png`, dan `application/pdf` dengan ukuran maksimal 5MB

#### Scenario: Kategori training-certificate
- **WHEN** klien meminta upload dengan kategori `training-certificate`
- **THEN** sistem mengizinkan MIME `image/jpeg`, `image/png`, dan `application/pdf` dengan ukuran maksimal 10MB

#### Scenario: Kategori article-photo
- **WHEN** klien meminta upload dengan kategori `article-photo`
- **THEN** sistem mengizinkan MIME `image/jpeg` dan `image/png` dengan ukuran maksimal 5MB

### Requirement: Prosedur penambahan kategori baru

Setiap penambahan kategori storage baru SHALL melalui change OpenSpec formal yang merujuk spec ini. Penambahan kategori SHALL mencakup: (a) nama kategori (kebab-case), (b) batas ukuran maksimum (dalam byte), (c) whitelist MIME yang diizinkan, dan (d) referensi ke domain pemilik.

#### Scenario: Kategori baru ditambahkan melalui change OpenSpec
- **WHEN** sebuah change membutuhkan kategori storage baru
- **THEN** change tersebut mendokumentasikan nama, batas ukuran, whitelist MIME, dan domain pemilik di design.md atau spec delta

#### Scenario: Kategori tidak dikenal ditolak
- **WHEN** klien meminta upload dengan nama kategori yang tidak terdaftar
- **THEN** sistem menolak dengan `StorageClientError::InvalidMime`
