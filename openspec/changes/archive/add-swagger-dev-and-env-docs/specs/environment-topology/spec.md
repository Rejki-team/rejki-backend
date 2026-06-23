## ADDED Requirements

### Requirement: Tiga environment dengan satu basis konfigurasi

Proyek SHALL mendefinisikan tiga konteks menjalankan aplikasi: (1) **local dev** di mesin pengembang via Docker Compose (`APP_ENV=development`), (2) **dev di VPS**, dan (3) **prod di VPS**. Dev dan prod di VPS SHALL dijalankan sebagai dua instance Docker Compose terpisah (direktori, project name, port host, dan database berbeda) pada satu VPS yang sama. Struktur variabel konfigurasi antar environment SHALL pada dasarnya identik; yang berbeda hanya nilainya.

#### Scenario: Dev dan prod hidup berdampingan di satu VPS
- **WHEN** operator menjalankan instance dev (`rejki-dev`) dan prod (`rejki-prod --profile prod`) di satu VPS
- **THEN** keduanya berjalan tanpa bentrok nama container, port host, maupun database

#### Scenario: Basis konfigurasi sama
- **WHEN** seseorang menyiapkan `.env` untuk dev maupun prod
- **THEN** kumpulan variabel yang dibutuhkan sama; hanya nilai (mis. `APP_ENV`, domain, kredensial) yang berbeda

### Requirement: Pembeda dev versus prod terdokumentasi

Dokumentasi proyek SHALL menyatakan secara eksplisit pembeda mengikat antara environment dev dan prod di VPS: (a) **dev memakai subdomain** (mis. `dev.rejki.id`) sedangkan **prod memakai domain utama** (mis. `rejki.id`); (b) **dev menyajikan Swagger UI** sedangkan **prod tidak**; (c) konfigurasi nginx dapat sedikit berbeda (mis. rate limiting & logging di prod). Selain pembeda ini, dev dan prod diperlakukan setara.

#### Scenario: Pembaca dokumentasi memahami pembeda
- **WHEN** seorang pengembang membaca dokumentasi deployment/environment
- **THEN** ia dapat mengidentifikasi bahwa dev = subdomain + Swagger, prod = domain utama tanpa Swagger, dan sisanya setara

#### Scenario: Akses production tidak mengekspos alat dev
- **WHEN** aplikasi diakses melalui domain produksi
- **THEN** tidak ada Swagger UI maupun perkakas khusus development yang terekspos

### Requirement: Secret dan variable CI/CD seragam antar environment

Karena basis konfigurasi sama, secret dan variable pada GitHub Actions SHALL seragam strukturnya untuk dev dan prod; yang membedakan hanya nilai per-environment (mis. domain/subdomain, `APP_ENV`, token tunnel). Dokumentasi CI/CD SHALL menyatakan hal ini agar tidak terjadi duplikasi konfigurasi yang menyimpang.

#### Scenario: Menyiapkan deploy environment baru
- **WHEN** operator menyiapkan pipeline untuk environment dev atau prod
- **THEN** ia memakai kumpulan secret/variable yang sama dan hanya mengganti nilai yang environment-specific

#### Scenario: Konsistensi terjaga
- **WHEN** sebuah variable konfigurasi ditambahkan untuk satu environment
- **THEN** dokumentasi mengarahkan agar variable yang sama tersedia di environment lain (dengan nilai sesuai) untuk menjaga paritas
