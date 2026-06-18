# PRD — Rejki Web Dashboard (Admin)

| | |
|---|---|
| **Aplikasi** | Rejki Web Dashboard (admin & moderasi) |
| **Dokumen** | Sub-PRD (bagian dari [rejki-prd.md](rejki-prd.md)) |
| **Versi** | 1.0 — Final |
| **Tanggal** | 2026-06-16 |
| **Status** | Semua backend selesai, 13 change terverifikasi, siap implementasi UI |
| **Stack klien** | Vue.js 3 + TypeScript + Tailwind CSS (dikonfirmasi pemilik produk) |
| **Repositori klien** | `rejki-web/` (greenfield, sibling dari `rejki-backend/`) |
| **Pemilik** | _(belum ditentukan)_ |

> **Perubahan dari v0.1 → v0.2.** Versi 0.1 adalah _placeholder_ (semua "USULAN" generik). Versi ini **menggantikannya** dengan spesifikasi nyata yang diturunkan langsung dari **User Story pemilik produk** (halaman per halaman, menu per menu). Setiap requirement fungsional (FR) ditelusuri ke User Story dan diberi **status backend** (✅ ada / 🔧 sebagian / 📋 baru) berbasis penelusuran kode aktual di `rust-services/` per **2026-06-14**, serta dirujuk ke **proposal OpenSpec** yang menjadi kontrak implementasinya.

> **Akurasi.** Klaim "sudah ada di backend" merujuk file kode aktual (lihat [§9 Keterlacakan](#9-keterlacakan-fr--openspec--kode)). Klaim praktik standar industri (RBAC default-deny, masking PII + audit akses, keamanan sesi admin, UX tabel data) bersumber kredibel ([§10 Sumber](#10-sumber--rujukan)). Inferensi yang belum dikonfirmasi ditandai **_(USULAN)_** atau **_(TBD)_**.

> **⚠️ Koreksi 2026-06-15 (audit kode ulang).** Penelusuran kode aktual `rust-services/` menemukan **6 gap backend** yang sebelumnya keliru ditandai "✅ selesai" di v0.2. Detail & bukti baris kode di [dashboard-gap-analysis.md](../dashboard-gap-analysis.md). Ringkasnya: (1) **listing pengajuan KYC admin tidak ada**; (2) **admin baca dokumen KTP/Selfie pengguna lain tidak ada**; (3) **auto-purge dokumen saat KYC ditolak tidak ter-wire**; (4) **bulk suspend pengguna belum ada** (baru single); (5) **auto-purge saat suspend permanen tidak ada trigger**; (6) **model Barang Bekas masih jual-beli**, belum "gratis/donasi". Gap ditutup oleh tiga change baru: **`add-user-admin-management`** (1–3), **`extend-user-suspension-bulk-purge`** (4–5), **`extend-barang-bekas-gratis-model`** (6). **Update 2026-06-15: ketiga change backend kini SELESAI & terverifikasi** (clippy/fmt bersih, 44 test integration live lulus). Endpoint bulk suspend final: `POST /api/v1/auth/admin/users/suspend` (admin-protected). Status pada §4/§5/§8 di bawah telah dikoreksi.

---

## Daftar Isi
1. [Ringkasan & Tujuan](#1-ringkasan--tujuan)
2. [Peran & Model Akses (RBAC)](#2-peran--model-akses-rbac)
3. [Model Autentikasi Admin](#3-model-autentikasi-admin)
4. [Pemetaan Backend ↔ Web](#4-pemetaan-backend--web)
5. [Requirement Fungsional per Menu/Halaman](#5-requirement-fungsional-per-menuhalaman)
6. [Pola UX Lintas-Halaman (Global)](#6-pola-ux-lintas-halaman-global)
7. [Requirement Non-Fungsional](#7-requirement-non-fungsional)
8. [Kebutuhan Backend Baru (ringkas) & Status](#8-kebutuhan-backend-baru-ringkas--status)
9. [Keterlacakan (FR → OpenSpec → Kode)](#9-keterlacakan-fr--openspec--kode)
10. [Sumber & Rujukan](#10-sumber--rujukan)

---

## 1. Ringkasan & Tujuan

Rejki Web Dashboard adalah aplikasi web internal (**Vue.js + TypeScript + Tailwind CSS**) untuk **admin** menjaga kualitas konten dan mengelola operasional platform: meninjau & memverifikasi data diri (KYC) pengguna, memoderasi keempat vertikal iklan, mengelola siklus pelatihan (verifikasi, konfirmasi peserta, badge/sertifikat), menindak laporan/aduan pengguna, serta menerbitkan komunikasi korporat (blog/artikel) dengan broadcast notifikasi.

**Tujuan:**
- Menjaga kualitas & keamanan konten lintas empat vertikal + pelatihan.
- Memberi admin alat menindak pelanggaran secara cepat, **terdokumentasi**, dan **dapat diaudit** (alasan + bukti wajib pada aksi sensitif).
- Melindungi data sensitif pengguna (NIK, Foto KTP, Foto Selfie) dengan **masking** + **click-to-view yang teraudit** dan retensi sesuai UU PDP.
- Menyediakan jejak akuntabel atas setiap tindakan admin.

**Catatan klien-agnostik.** Backend bersifat REST/JSON + envelope `ApiResponse<T>` ([api-standard.html](../api-standard.html)); pilihan Vue tidak mengubah kontrak API. Dashboard adalah **SPA** yang mengonsumsi endpoint admin di bawah namespace `/api/v1/admin/**` (kecuali login admin di `/api/v1/auth/admin/login`).

---

## 2. Peran & Model Akses (RBAC)

User Story baru menyebut **satu role `admin`** yang dapat melakukan seluruh aksi moderasi. Backend saat ini **belum memiliki konsep `role`** sama sekali (lihat [§8](#8-kebutuhan-backend-baru-ringkas--status)). Maka:

| Keputusan | Isi |
|---|---|
| **Role awal** | Satu role `admin` (semua aksi sesuai User Story). Role pengguna umum tetap `user`. |
| **Desain extensible** | Kolom `role` dirancang dengan `CHECK` yang dapat diperluas (mis. kelak `super_admin`, `moderator`, `support`) **tanpa breaking change** — penambahan nilai enum & aturan otorisasi bersifat aditif. Tier majemuk yang diusulkan v0.1 ditunda, bukan dibuang. |
| **Prinsip** | **Default-deny** & **least-privilege**: setiap endpoint `/api/v1/admin/**` ditolak kecuali pembawa token ber-`role=admin`. Default settings deny unless explicitly permitted. ([OWASP Authorization Cheat Sheet][owasp-authz]) |
| **Akuntabilitas** | Setiap aksi sensitif (suspend, approve/reject, akses dokumen) tercatat (aktor, objek, waktu, alasan/bukti). |

> **Pemetaan ke kapabilitas OpenSpec:** `add-admin-rbac` → `admin-authentication`, `role-authorization`.

---

## 3. Model Autentikasi Admin

Diturunkan dari User Story: _"login jika akun sudah diproses DBA dengan inject data"_, _"login menggunakan Email & Password terdaftar"_, _"melihat foto profil, nama, role"_, _"logout via menu profil"_.

| ID | Requirement | Prioritas | Status backend |
|---|---|---|---|
| FR-ADM-AUTH-01 | Login admin dengan **email + password** (tanpa OTP) | M | 📋 Baru — `add-admin-rbac` |
| FR-ADM-AUTH-02 | Akun admin **di-inject oleh DBA** (seed/migrasi); **tidak ada self-register** admin | M | 📋 Baru — `add-admin-rbac` |
| FR-ADM-AUTH-03 | Hanya token ber-`role=admin` yang boleh mengakses dashboard & endpoint `/admin/**` | M | 📋 Baru — `add-admin-rbac` |
| FR-ADM-AUTH-04 | Tampilkan **foto profil, nama, role** admin (header) | M | 🔧 Sebagian — profil ada; `role` ditambah |
| FR-ADM-AUTH-05 | **Logout** dari menu yang muncul saat klik susunan foto/nama/role | M | ✅ Ada (`POST /api/v1/auth/logout`) + guard klien |

**Catatan desain auth.** Login pengguna reguler saat ini sudah berbasis **password** (`POST /api/v1/auth/login`, body `email`+`password` — `auth-service/src/application/dto.rs`), bukan OTP. Untuk admin disediakan **endpoint terpisah** `POST /api/v1/auth/admin/login` yang memverifikasi kredensial **dan** `role=admin`, lalu menerbitkan JWT RS256 dengan klaim `role` ditambahkan (aditif terhadap `JwtClaims`/`AuthClaims`). Memisahkan endpoint memudahkan kebijakan sesi admin yang lebih ketat (§7) tanpa menyentuh alur mobile.

> Akun admin yang di-inject DBA tetap melewati state akun yang ada; admin diasumsikan langsung `active` (tidak menjalani KYC). Detail di `add-admin-rbac/design.md`.

---

## 4. Pemetaan Backend ↔ Web

Tabel ini adalah inti permintaan: **mana pekerjaan rejki-backend, mana rejki-web.**

| Area User Story | Sudah ada di backend? | Pekerjaan **rejki-backend** | Pekerjaan **rejki-web** | OpenSpec |
|---|---|---|---|---|
| Login admin (email+password, inject DBA, role) | ✅ | Kolom `role`; `POST /auth/admin/login`; middleware `require_admin`; seed admin | Halaman login; auth store; route guard | `add-admin-rbac` |
| Profil admin (foto/nama/role) + logout | ✅ | Sertakan `role` pada klaim & `GET /me` | Header profil + dropdown logout | `add-admin-rbac` |
| Verifikasi KYC (NIK mask + click-to-view teraudit, KTP/selfie, approve/reject, hapus dok saat ditolak, notifikasi email+in-app) | 🔧 review+notif ✅; **listing/baca-dokumen/auto-purge 🔴 GAP** | **Listing KYC admin**, **endpoint admin baca dokumen** teraudit, **auto-purge saat ditolak** (3 gap) | Halaman Pengelolaan Pengguna; popup detail; UI masking + click-to-view | `add-user-admin-management` (baru) + `add-user-service-kyc` |
| Suspend **pengguna** (sementara/permanen, alasan+bukti ≤5MB, hapus dok jika permanen, notifikasi email+in-app) | ✅ single + **bulk** + **purge-permanen** DONE (`POST /auth/admin/users/suspend` partial-success; notif email+in-app) | — (BE selesai) | UI suspend (single/bulk, upload bukti) | `extend-user-suspension-bulk-purge` ✅ + `extend-auth-service-onboarding` |
| Suspend **per-IKLAN** (4 vertikal; single/sebagian/sekaligus; alasan+bukti; notifikasi) | ✅ | State moderasi iklan + suspend per-iklan + bukti presigned | UI ceklis + tombol Suspend + popup sementara/permanen | `extend-iklan-moderation` |
| Daftar iklan (tabel responsif, **foto popup**, **search**, **sort by status**, **Export CSV**) | ✅ | Query search/filter/sort; surface `foto_urls`; endpoint export CSV | Tabel dinamis; popup foto; input search; dropdown sort; tombol Export CSV | `extend-iklan-moderation` |
| **Barang Bekas — model "Gratis"** (Jenis Barang, Jumlah, Lokasi Pengambilan, status Sudah Diambil) | 🔴 GAP (entity masih jual-beli: `harga`/`kondisi`/`is_sold`) | Ubah model → gratis/donasi; surface kolom baru di listing/CSV | Kolom tabel sesuai User Story | `extend-barang-bekas-gratis-model` (baru) |
| **Pelatihan — Daftar**: auto-approve (admin), verifikasi pengajuan (user), edit/cancel milik admin, 7 status | ✅ | Status moderasi pelatihan; `created_by_role`; create/edit/cancel admin; review user-submitted | Tabel; popup detail; form Tambah/Edit; tombol Tolak/Terima/Batalkan | `add-pelatihan-enrollment-badge` |
| **Pelatihan — Konfirmasi** (bukti transfer, approve/reject, notifikasi) | ✅ | Domain enrollment + `bukti_transfer` object key + review admin | Tabel; popup detail; tombol Tolak/Terima | `add-pelatihan-enrollment-badge` |
| **Pelatihan — Badge** (sertifikat, tgl disetujui, verifikasi admin) | ✅ | Domain badge + sertifikat object key + review admin | Tabel; popup detail; tombol Tolak/Terima | `add-pelatihan-enrollment-badge` |
| Pengelolaan Dukungan (Aduan konten/pengguna; popup; tindak lanjut; notifikasi) | ✅ | Domain report/aduan + foto bukti + tindak lanjut + notifikasi (via `report-service`) | Tabel; popup detail; tombol Tolak/Terima | `add-content-reports` |
| Corporate Communication (artikel: list/create/edit; kategori; foto; broadcast notifikasi ke semua user) | ✅ | Domain artikel + broadcast in-app (reuse `send_bulk`); soft-delete; presigned foto `article-photo` | Tabel; editor artikel; popup; tombol Batalkan/Simpan | `add-corporate-comms` |
| Sidebar collapse, loading progress, pop-up status, responsif | ❌ (murni UI) | — (kontrak API standar sudah cukup) | Layout sidebar; komponen loading/toast/modal global | `add-rejki-web-dashboard` |

Legenda status backend: **✅** terverifikasi ada di kode · **🔧** sebagian ada · **❌/📋** belum ada (baru).

---

## 5. Requirement Fungsional per Menu/Halaman

> **Prioritas (MoSCoW):** M = Must, S = Should, C = Could, W = Won't (sekarang).
> Setiap baris **status** = status backend; pekerjaan UI selalu di rejki-web (`add-rejki-web-dashboard`).

### 5.0 Sidebar & Navigasi

| ID | Requirement | Prio | Status |
|---|---|---|---|
| FR-ADM-NAV-01 | Buka/tutup sidebar via ikon hamburger | M | 📋 (web) |
| FR-ADM-NAV-02 | Daftar menu vertikal + deskripsi: Iklan Pekerja, Iklan Pekerjaan, Iklan Pelatihan, Iklan Barang Bekas Gratis, Pengelolaan Pengguna, Pengelolaan Dukungan, Corporate Communication | M | 📋 (web) |
| FR-ADM-NAV-03 | Sub-menu pada **Iklan Pelatihan**: Daftar Pelatihan, Konfirmasi Pelatihan, Badge Pelatihan | M | 📋 (web) |

### 5.1 Iklan Pekerja (Menu: Iklan Pekerja)

| ID | Requirement | Prio | Status |
|---|---|---|---|
| FR-ADM-WRK-01 | Tabel daftar Iklan Pekerja per-halaman, responsif & dinamis (kolom: ID, Nama Pekerja, Pengalaman Kerja, Upah, Jam Kerja, Cara Menghubungi, Foto Pekerjaan, Status) | M | 🔧 list ada; kolom/pagination diperluas — `extend-iklan-moderation` |
| FR-ADM-WRK-02 | Popup tampil jelas tiap Foto Pekerjaan saat diklik | M | 🔧 (`foto_urls` di-surface) — `extend-iklan-moderation` |
| FR-ADM-WRK-03 | Pencarian by Nama / Kode / Pembuat | M | 📋 `extend-iklan-moderation` |
| FR-ADM-WRK-04 | Urutkan by status (dropdown) | S | 📋 `extend-iklan-moderation` |
| FR-ADM-WRK-05 | Export CSV | S | 📋 `extend-iklan-moderation` |
| FR-ADM-WRK-06 | Suspend (ceklis → tombol Suspend → popup sementara/permanen) single/sebagian/sekaligus; **wajib alasan + bukti** (gambar/PDF); notifikasi email + in-app | M | 📋 `extend-iklan-moderation` |
| FR-ADM-WRK-07 | **Tidak** menampilkan data sensitif (NIK, Foto KTP, Foto Selfie) di halaman ini | M | ✅ (data tsensitif terpisah di user-service; iklan tak memuatnya) |
| FR-ADM-WRK-08 | Admin **tidak dapat mengubah** data iklan terdaftar (read-only kecuali suspend) | M | ✅/📋 (tak ada endpoint edit admin; dijamin RBAC) |

### 5.2 Iklan Pekerjaan (Menu: Iklan Pekerjaan)

| ID | Requirement | Prio | Status |
|---|---|---|---|
| FR-ADM-JOB-01 | Tabel daftar (kolom: ID, Judul, Deskripsi, Upah, Jam Kerja, Foto Pekerjaan, Status) | M | 🔧 — `extend-iklan-moderation` |
| FR-ADM-JOB-02 | Popup Foto Pekerjaan | M | 🔧 — `extend-iklan-moderation` |
| FR-ADM-JOB-03 | Pencarian by Judul / Kode / Pembuat | M | 📋 `extend-iklan-moderation` |
| FR-ADM-JOB-04 | Urutkan by status | S | 📋 `extend-iklan-moderation` |
| FR-ADM-JOB-05 | Export CSV | S | 📋 `extend-iklan-moderation` |
| FR-ADM-JOB-06 | Suspend (sama pola WRK-06) | M | 📋 `extend-iklan-moderation` |
| FR-ADM-JOB-07 | Tanpa data sensitif; read-only (tak boleh edit) | M | ✅/📋 |

### 5.3 Iklan Pelatihan — Sub-Menu **Daftar Pelatihan**

| ID | Requirement | Prio | Status |
|---|---|---|---|
| FR-ADM-TRN-01 | Tabel daftar (kolom: ID, Judul, Deskripsi, Penyelenggara, Lokasi, Tanggal, Jumlah Peserta, Status) + ikon Mata per baris | M | 📋 `add-pelatihan-enrollment-badge` |
| FR-ADM-TRN-02 | **7 status** pelatihan: Verifikasi Tertunda / Verifikasi Dalam Proses / Verifikasi Ditolak / Verifikasi Diterima / Pelatihan Berjalan / Pelatihan Selesai / Pelatihan Belum Dimulai | M | 📋 `add-pelatihan-enrollment-badge` |
| FR-ADM-TRN-03 | **Tambah Pelatihan** (admin) → form popup → **auto-approve** | M | 📋 `add-pelatihan-enrollment-badge` |
| FR-ADM-TRN-04 | Popup detail via ikon Mata; tombol kontekstual: **Tolak/Terima** bila dibuat **user**; **Batalkan/Edit/Simpan** bila dibuat **admin** | M | 📋 `add-pelatihan-enrollment-badge` |
| FR-ADM-TRN-05 | **Edit** hanya pelatihan milik admin; **tidak** boleh edit milik user | M | 📋 `add-pelatihan-enrollment-badge` |
| FR-ADM-TRN-06 | **Verifikasi pengajuan user**: wajib alasan bila tolak; tombol Tolak/Terima **terkunci** setelah verifikasi dilakukan; notifikasi otomatis | M | 📋 `add-pelatihan-enrollment-badge` |
| FR-ADM-TRN-07 | Pencarian by Judul / Kode ID / Penyelenggara; sort by status; Export CSV | S | 📋 `add-pelatihan-enrollment-badge` |

### 5.4 Iklan Pelatihan — Sub-Menu **Konfirmasi Pelatihan**

| ID | Requirement | Prio | Status |
|---|---|---|---|
| FR-ADM-CNF-01 | Tabel (kolom: ID Pelatihan, ID Pengguna, Judul, Penyelenggara, Tanggal, **Foto Bukti Transfer**, Status) + ikon Mata | M | 📋 `add-pelatihan-enrollment-badge` |
| FR-ADM-CNF-02 | Popup detail; tombol **Tolak/Terima**; wajib alasan bila tolak; terkunci setelah verifikasi | M | 📋 `add-pelatihan-enrollment-badge` |
| FR-ADM-CNF-03 | Status: Tertunda / Dalam Proses / Ditolak / Diterima | M | 📋 `add-pelatihan-enrollment-badge` |
| FR-ADM-CNF-04 | Notifikasi otomatis ke pengguna atas hasil verifikasi | M | 📋 `add-pelatihan-enrollment-badge` |
| FR-ADM-CNF-05 | Tidak dapat menyunting konfirmasi; Pencarian/sort/Export CSV | S | 📋 `add-pelatihan-enrollment-badge` |

### 5.5 Iklan Pelatihan — Sub-Menu **Badge Pelatihan**

| ID | Requirement | Prio | Status |
|---|---|---|---|
| FR-ADM-BDG-01 | Tabel (kolom: ID Pelatihan, ID Pengguna, Nama Pengguna, Judul, Penyelenggara, Tanggal Badge Disetujui, **Sertifikat Pelatihan**, Status Verifikasi Admin) + ikon Mata | M | 📋 `add-pelatihan-enrollment-badge` |
| FR-ADM-BDG-02 | Popup detail; tombol **Tolak/Terima**; wajib alasan bila tolak; terkunci setelah verifikasi | M | 📋 `add-pelatihan-enrollment-badge` |
| FR-ADM-BDG-03 | Status: Tertunda / Dalam Proses / Ditolak / Diterima | M | 📋 `add-pelatihan-enrollment-badge` |
| FR-ADM-BDG-04 | Notifikasi otomatis ke pengguna; tidak dapat menyunting; Pencarian/sort/Export CSV | S | 📋 `add-pelatihan-enrollment-badge` |

### 5.6 Iklan Barang Bekas Gratis (Menu)

| ID | Requirement | Prio | Status |
|---|---|---|---|
| FR-ADM-GDS-01 | Tabel (kolom: ID, Judul, Deskripsi, **Jenis Barang** Bekas/Baru, Foto Barang, Jumlah, Lokasi Pengambilan, Status: Baru/Sudah Diambil/Suspended) | M | 🔴 GAP — entity masih jual-beli (`harga`/`kondisi`/`is_sold`); ubah ke model gratis — `extend-barang-bekas-gratis-model` |
| FR-ADM-GDS-02 | Popup Foto Barang | M | 🔧 (`foto_urls` di-surface) — `extend-iklan-moderation` |
| FR-ADM-GDS-03 | Pencarian by Judul / Kode | M | 📋 `extend-iklan-moderation` |
| FR-ADM-GDS-04 | Urutkan by status; Export CSV | S | 📋 `extend-iklan-moderation` |
| FR-ADM-GDS-05 | Suspend (pola WRK-06) | M | 📋 `extend-iklan-moderation` |
| FR-ADM-GDS-06 | Tanpa data sensitif; read-only (tak boleh edit) | M | ✅/📋 |

> Catatan: nama menu "Barang Bekas **Gratis**" menyiratkan model gratis/donasi (bukan jual-beli berharga). Entity backend saat ini bermodel jual-beli (`harga`, `kondisi`, `is_sold` — [entity.rs:45-49](../../rust-services/iklan-barang-bekas-service/src/domain/entity.rs#L45)). **Keputusan pemilik produk (2026-06-15): ubah ke model gratis/donasi** — hapus `harga`/`kondisi`, tambah `jenis_barang`, `jumlah`, `lokasi_pengambilan`, status `availability_status` (Tersedia/Sudah Diambil). Ditangani di change baru **`extend-barang-bekas-gratis-model`**.

### 5.7 Pengelolaan Pengguna (Menu)

| ID | Requirement | Prio | Status |
|---|---|---|---|
| FR-ADM-USR-01 | Tabel pengajuan verifikasi data diri (kolom: ID, Nama, Tingkat Pendidikan, Jenis Kelamin, TTL, Alamat Domisili, Kombinasi wilayah Kelurahan→Negara, Status Verifikasi) + ikon Mata | M | 🔴 GAP — **listing admin belum ada** (hanya `POST /admin/kyc/{id}/review`) — `add-user-admin-management` |
| FR-ADM-USR-02 | Popup detail termasuk **NIK** (masking + **click-to-view teraudit**), Foto KTP & Foto Selfie (masking + click-to-view teraudit) | M | 🔴 GAP — NIK mask & `document_access_log` ✅, tetapi **admin baca dokumen pengguna lain belum ada** (`/me/documents` self-only) — `add-user-admin-management` |
| FR-ADM-USR-03 | Tombol **Tolak/Terima**; wajib alasan bila tolak; **terkunci** setelah verifikasi | M | ✅ (`POST /users/admin/kyc/{id}/review`); penguncian pasca-verifikasi dilengkapi `add-user-admin-management` |
| FR-ADM-USR-04 | Notifikasi hasil verifikasi otomatis (email + in-app) | M | ✅ ([service.rs:290-295](../../rust-services/user-service/src/application/service.rs#L290)) |
| FR-ADM-USR-05 | Foto KTP/Selfie **bertahan selama akun aktif**; **dihapus otomatis saat penolakan** | M | 🔴 GAP — `purge_documents` ada tapi **tak ter-wire ke reject** ([service.rs:427-428](../../rust-services/user-service/src/application/service.rs#L427)) — `add-user-admin-management` |
| FR-ADM-USR-06 | Akses **terbatas** (tak bisa kembali meninjau setelah verifikasi selesai) | M | 🔧 status review ada; **guard idempotensi** dilengkapi — `add-user-admin-management` + guard UI |
| FR-ADM-USR-07 | **Suspend pengguna** (sementara/permanen) single/sebagian/sekaligus; **alasan + bukti ≤5MB (1 file)**; **Foto KTP/Selfie dihapus otomatis jika suspend permanen**; notifikasi email + in-app | M | ✅ DONE — single + **bulk** (`POST /auth/admin/users/suspend`, partial-success) + **purge permanen** (via `UserClient`) + notif **email+in-app** — `extend-user-suspension-bulk-purge` |
| FR-ADM-USR-08 | Pencarian by ID / Nama; sort by status; Export CSV; tak boleh ubah data | S | 🔴 GAP — bagian dari admin listing — `add-user-admin-management` |

### 5.8 Pengelolaan Dukungan (Menu)

| ID | Requirement | Prio | Status |
|---|---|---|---|
| FR-ADM-SUP-01 | Tabel Aduan (kolom: ID Pengaduan, Tanggal, ID Pelapor, ID Iklan diadukan, Keterangan, Status) + ikon Mata | M | ✅ `add-content-reports` |
| FR-ADM-SUP-02 | Popup detail (+ **Foto Bukti**); admin **wajib mengisi tindakan**; tombol Tolak/Terima; terkunci setelah verifikasi | M | ✅ `add-content-reports` |
| FR-ADM-SUP-03 | Status: Tertunda / Dalam Proses / Ditolak / Diterima | M | ✅ `add-content-reports` |
| FR-ADM-SUP-04 | Notifikasi otomatis (email + in-app) ke pihak terdampak | M | ✅ `add-content-reports` |
| FR-ADM-SUP-05 | Pencarian by ID Pengguna / ID Pengaduan; sort by status; Export CSV; tak boleh ubah data aduan | S | ✅ `add-content-reports` |

> **Prasyarat lintas-aplikasi:** pembuatan aduan dilakukan dari **Rejki Mobile** (pengguna melaporkan iklan). Lihat [prd-mobile.md §5](prd-mobile.md).

### 5.9 Corporate Communication (Menu)

| ID | Requirement | Prio | Status |
|---|---|---|---|
| FR-ADM-COM-01 | Tabel Unggahan (kolom: ID Artikel, Pembuat [read-only dari sesi], Kategori, Judul, Isi, Foto) + ikon Pena & ikon Tong Sampah per baris | M | 📋 `add-corporate-comms` |
| FR-ADM-COM-02 | **Buat Artikel** (Judul, Isi, Kategori [dropdown: saat ini hanya `informasi`], Foto); tombol Batalkan/Simpan | M | 📋 `add-corporate-comms` |
| FR-ADM-COM-03 | **Lihat/Sunting** via ikon Pena (+ timestamps dibuat/diperbarui); pembuat read-only | M | 📋 `add-corporate-comms` |
| FR-ADM-COM-04 | **Hapus** via ikon Tong Sampah | S | 📋 `add-corporate-comms` |
| FR-ADM-COM-05 | **Broadcast notifikasi in-app ke seluruh pengguna** saat artikel baru terbit atau artikel lama diperbarui | M | 📋 `add-corporate-comms` (reuse `send_bulk`) |
| FR-ADM-COM-06 | Pencarian by Judul; sort by Kategori | S | 📋 `add-corporate-comms` |

---

## 6. Pola UX Lintas-Halaman (Global)

Berlaku untuk **semua** halaman (pekerjaan rejki-web; spec di `add-rejki-web-dashboard`):

| ID | Requirement | Prio | Catatan & sumber |
|---|---|---|---|
| FR-ADM-UX-01 | **Loading progress** saat sistem memproses data | M | Skeleton/spinner; "blank table while fetching is jarring" ([NNG via Eleken][ux-table]) |
| FR-ADM-UX-02 | **Pop-up status** saat proses selesai (berhasil/gagal) | M | Toast/modal hasil |
| FR-ADM-UX-03 | Tabel **responsif & dinamis**, per-halaman | M | Server-side pagination untuk dataset besar ([UX table best practice][ux-table]) |
| FR-ADM-UX-04 | **Server-side** search/sort/filter & pagination (bukan client-side untuk dataset besar) | S | "For any dataset above ~200 rows, use server-side pagination" ([ux-table]) |
| FR-ADM-UX-05 | **Masking + click-to-view** data sensitif dengan **audit setiap pembukaan** | M | Dynamic masking + log akses ([PII masking & audit][pii-mask]); didukung `document_access_log` |
| FR-ADM-UX-06 | Aksi sensitif **mengunci** kembali setelah selesai (tak bisa diulang) | M | Idempotensi + integritas audit |
| FR-ADM-UX-07 | Bulk select (ceklis) untuk suspend massal | S | "Acting on data → bulk action areas" ([ux-table]) |

---

## 7. Requirement Non-Fungsional

| Kategori | Requirement | Sumber |
|---|---|---|
| **Otorisasi** | RBAC **default-deny** di setiap endpoint `/admin/**`; least-privilege; ownership/role divalidasi server-side | [OWASP Authorization Cheat Sheet][owasp-authz] |
| **Audit trail** | Setiap aksi sensitif (suspend, approve/reject, akses dokumen) tercatat append-only (aktor, objek, waktu, alasan/bukti); audit "should be a safety net, not a scavenger hunt" | [PII/Audit best practice][pii-mask] |
| **Masking PII** | NIK & dokumen ter-mask default; **click-to-view** membuka penuh **dan mencatat akses**; data asli tak pernah di-log | [PII masking & audit logging][pii-mask] |
| **Sesi admin** | Idle timeout pendek (mis. **2–5 menit** untuk aplikasi nilai-tinggi), absolute timeout; logout invalidasi sisi server; HTTPS + cookie `Secure`/`HttpOnly`/`SameSite`; MFA **_(USULAN)_** | [OWASP Session Management Cheat Sheet][owasp-session] |
| **Keamanan baseline** | Mengikuti baseline Phase 1.5 yang sudah ada | [security-baseline.html](../security-baseline.html) |
| **Observability** | Aksi admin masuk logging terstruktur dengan propagasi `request_id` | [logging-standard.html](../logging-standard.html) |
| **Retensi PDP** | Dokumen KYC: simpan selama akun aktif, **hapus otomatis saat penolakan/suspend permanen/penutupan akun** | UU PDP; `add-user-service-kyc` (K11) |
| **Responsif** | Kontrol pagination & layout beradaptasi mobile/desktop | [UX table best practice][ux-table] |

---

## 8. Kebutuhan Backend Baru (ringkas) & Status

Diturunkan dari penelusuran kode aktual (`rust-services/`). **Pembaruan 2026-06-16 (final):** Semua backend dashboard telah selesai. Seluruh 16 OpenSpec changes terverifikasi (clippy/fmt/test bersih), 13 di-archive. Tidak ada gap tersisa.

| # | Kebutuhan backend | OpenSpec change | Status sumber |
|---|---|---|---|
| 1 | **RBAC**: kolom `role`, login admin, middleware `require_admin`, seed admin | `add-admin-rbac` | ✅ selesai |
| 2 | **Moderasi iklan** 4 vertikal: status moderasi, suspend per-iklan + bukti, search/sort, `foto_urls` API, CSV export | `extend-iklan-moderation` | ✅ selesai |
| 3 | **Pelatihan**: 7 status moderasi, auto-approve admin vs review user, enrollment + bukti transfer, badge + sertifikat | `add-pelatihan-enrollment-badge` | ✅ selesai |
| 4 | **Report/Aduan**: domain pelaporan + tindak lanjut admin + notifikasi | `add-content-reports` | ✅ selesai (lihat `report-service` + `report-service-client`) |
| 5 | **Corporate Communication**: domain artikel + broadcast notifikasi | `add-corporate-comms` | ✅ selesai |
| 6 | **KYC review oleh admin** (approve/reject + notifikasi) | `add-user-service-kyc` + `add-admin-rbac` | ✅ selesai |
| 6b | **Listing KYC admin + baca dokumen teraudit + auto-purge saat reject** | **`add-user-admin-management`** | ✅ selesai (gap #1–3) |
| 7 | **Suspend pengguna single + bukti** | `extend-auth-service-onboarding` + `add-admin-rbac` | ✅ selesai |
| 7b | **Bulk suspend pengguna + purge dokumen saat permanen** | **`extend-user-suspension-bulk-purge`** | ✅ selesai (gap #4–5) |
| 7c | **Model Barang Bekas Gratis** (jenis/jumlah/lokasi pengambilan/Sudah Diambil) | **`extend-barang-bekas-gratis-model`** | ✅ selesai (gap #6) |
| 8 | **Spesifikasi SPA dashboard** (UI/UX, konsumsi API, guard RBAC) | `add-rejki-web-dashboard` | ✅ selesai |
| 9 | **Storage service spec** (dokumentasi formal kontrak & 8 kategori) | `add-storage-service-spec` | ✅ selesai |
| 10 | **Region service** (data wilayah 4 tingkat + cascading API) | `add-region-service` | ✅ selesai |

> Dependensi: #2–#5 bergantung pada #1 (RBAC) untuk proteksi endpoint admin. #3 (enrollment/badge) dan #4 (report) memiliki **prasyarat sisi mobile** (dibuat/diajukan pengguna).

---

## 9. Keterlacakan (FR → OpenSpec → Kode)

Keterlacakan dua arah: tiap FR di atas mereferensikan **change OpenSpec**; tiap change mereferensikan balik **FR PRD** di `proposal.md`-nya. Klaim "✅ ada" ditelusuri ke kode aktual:

| Klaim "sudah ada" | Bukti kode |
|---|---|
| Login pakai password (bukan OTP) | `auth-service/src/application/dto.rs` (`LoginInput { email, password }`); router `POST /api/v1/auth/login` |
| Logout invalidasi refresh token | `auth-service` `POST /api/v1/auth/logout` |
| KYC review admin (approve/reject) | `user-service` `POST /api/v1/users/admin/kyc/{id}/review`; `add-user-service-kyc/tasks.md` §7 |
| NIK ter-mask + audit akses dokumen | `user_svc.profiles.nik_last4`; tabel `user_svc.document_access_log`; `add-user-service-kyc/tasks.md` §11 |
| Suspend + bukti (evidence_object_key) | `auth.account_suspension.evidence_object_key`; `add-user-service-kyc/tasks.md` §10 |
| Notifikasi email + in-app | `NotificationClient` (`send`/`send_bulk`/`send_email`); `add-user-service-kyc` §8 |
| `foto_urls` ada di barang-bekas (DB) | `iklan-barang-bekas-service/src/domain/entity.rs` (`foto_urls: Vec<String>`, kolom DB, belum di-surface ke API) |
| Tidak ada `role` di mana pun | `auth-service/src/infrastructure/jwt.rs` (`JwtClaims` tanpa `role`); `auth-service-client` (`AuthClaims` tanpa `role`) |
| Report/Aduan + bukti + tindak lanjut admin + notifikasi | `report-service` (14 file, 4 layer); `report-service-client` (trait `ReportClient`); migrasi schema `report`; `add-content-reports/tasks.md` §1–§10 |
| Corporate comms artikel + broadcast | `corporate-comms-service`; migrasi schema `comms`; `add-corporate-comms/tasks.md` §1–§7 |
| Middleware `require_admin` | `common/auth-middleware/src/lib.rs`; `add-admin-rbac/design.md` D4 |

> Catatan akurasi: beberapa field iklan (`lokasi`, `gaji_*`, `harga`, `tanggal_*`) **ada di entity tetapi belum di-`INSERT`** pada service layer saat ini — ini diperbaiki sebagai bagian `extend-iklan-moderation` / `add-pelatihan-enrollment-badge`. Lihat catatan di masing-masing `design.md`.

---

## 10. Sumber & Rujukan

**Internal (kode & standar Phase 1.5):**
[dashboard-gap-analysis.md](../dashboard-gap-analysis.md) (audit gap 2026-06-15) · [rejki-prd.md](rejki-prd.md) · [prd-mobile.md](prd-mobile.md) · [prd-ceo.md](prd-ceo.md) · [api-standard.html](../api-standard.html) · [security-baseline.html](../security-baseline.html) · [authorization-pattern.html](../authorization-pattern.html) · [logging-standard.html](../logging-standard.html) · [notification-contract.html](../notification-contract.html)

**OpenSpec changes terkait:** `add-admin-rbac`, `extend-iklan-moderation`, `add-pelatihan-enrollment-badge`, `add-content-reports`, `add-corporate-comms`, `add-rejki-web-dashboard`, `add-user-service-kyc`, `extend-auth-service-onboarding`, `add-region-service`.

**Eksternal (praktik standar industri):**

[owasp-authz]: https://cheatsheetseries.owasp.org/cheatsheets/Authorization_Cheat_Sheet.html
[owasp-session]: https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html
[pii-mask]: https://zuplo.com/learning-center/protect-sensitive-data-in-api-logs
[ux-table]: https://www.eleken.co/blog-posts/table-design-ux

- [OWASP Authorization Cheat Sheet][owasp-authz] — default-deny, least-privilege, RBAC.
- [OWASP Session Management Cheat Sheet][owasp-session] — idle/absolute timeout, cookie flags, logout invalidation.
- [Protecting Sensitive Data in API Logs — Zuplo][pii-mask] — masking PII + audit logging akses.
- [Table design UX guide — Eleken (mengutip Nielsen Norman Group)][ux-table] — server-side pagination, loading state, responsif, bulk action.

---

_Lihat juga: [rejki-prd.md](rejki-prd.md) (payung) · [prd-mobile.md](prd-mobile.md) · [prd-ceo.md](prd-ceo.md)._
