## ADDED Requirements

### Requirement: Dokumentasi API interaktif hanya pada development

Sistem SHALL menyajikan dokumentasi API interaktif (Swagger UI beserta dokumen OpenAPI) HANYA ketika `APP_ENV=development`. Ketika `APP_ENV=production`, rute Swagger UI dan endpoint dokumen OpenAPI SHALL TIDAK dipasang ke router sama sekali (gated saat pembentukan router, bukan sekadar disembunyikan atau diproteksi).

#### Scenario: Swagger tersedia di development
- **WHEN** aplikasi dijalankan dengan `APP_ENV=development` dan klien membuka rute Swagger UI
- **THEN** sistem menampilkan halaman Swagger UI yang memuat dokumen OpenAPI

#### Scenario: Swagger absen di production
- **WHEN** aplikasi dijalankan dengan `APP_ENV=production` dan klien meminta rute Swagger UI atau dokumen OpenAPI
- **THEN** sistem mengembalikan 404 (rute tidak terdaftar) dan tidak mengekspos dokumentasi API apa pun

#### Scenario: Endpoint bisnis tidak terpengaruh oleh gate
- **WHEN** aplikasi dijalankan pada environment mana pun
- **THEN** seluruh rute `/api/v1/*` dan `/health` berperilaku sama; hanya ketersediaan rute Swagger yang berbeda antar environment

### Requirement: Dokumen OpenAPI mendeskripsikan layanan

Sistem SHALL menyediakan dokumen OpenAPI yang memuat metadata layanan (judul, versi, deskripsi) dan setidaknya mendokumentasikan endpoint health-check. Dokumen SHALL dapat diperluas untuk endpoint lain tanpa mengubah mekanisme penyajian.

#### Scenario: Dokumen OpenAPI dapat diakses di development
- **WHEN** `APP_ENV=development` dan klien meminta dokumen OpenAPI (mis. `openapi.json`)
- **THEN** sistem mengembalikan dokumen OpenAPI valid yang memuat judul, versi, dan path health-check

#### Scenario: Versi dokumen selaras dengan aplikasi
- **WHEN** dokumen OpenAPI dihasilkan
- **THEN** metadata versi mencerminkan versi aplikasi/rilis sehingga konsumen dapat membedakan kontrak antar rilis

### Requirement: Dokumen OpenAPI tidak boleh bocor terpisah dari UI

Ketika dokumentasi API dinonaktifkan (production), endpoint dokumen OpenAPI (`openapi.json`) SHALL ikut tidak tersedia, tidak hanya halaman UI-nya. Skema API SHALL TIDAK dapat ditarik langsung melalui URL dokumen saat UI diblokir.

#### Scenario: openapi.json absen di production
- **WHEN** `APP_ENV=production` dan klien meminta URL dokumen OpenAPI (`/api-docs/openapi.json`) secara langsung
- **THEN** sistem mengembalikan 404 dan tidak membocorkan skema API apa pun

#### Scenario: UI dan dokumen di-gate sebagai satu kesatuan
- **WHEN** environment berubah antara development dan production
- **THEN** ketersediaan UI Swagger dan dokumen OpenAPI berubah bersama-sama (keduanya ada di dev, keduanya absen di prod)

### Requirement: Contoh anotasi endpoint tanpa membebani crate domain

Dokumen OpenAPI SHALL menyertakan minimal satu endpoint bisnis sebagai teladan (mis. `POST /api/v1/auth/login`) sehingga pola anotasi terbukti. Penyediaan dokumentasi ini SHALL TIDAK memaksa crate domain (mis. `auth-service`) bergantung pada pustaka dokumentasi; representasi skema untuk dokumentasi boleh didefinisikan di composition root.

#### Scenario: Endpoint auth terdokumentasi di dev
- **WHEN** `APP_ENV=development` dan dokumen OpenAPI dibuka
- **THEN** path `POST /api/v1/auth/login` tampil dengan skema request/response-nya

#### Scenario: Crate domain tetap bersih
- **WHEN** dependency crate domain (mis. `auth-service`) diperiksa
- **THEN** crate domain tidak menarik pustaka dokumentasi OpenAPI hanya untuk keperluan Swagger
