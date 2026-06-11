# PRD — Rejki Mobile

| | |
|---|---|
| **Aplikasi** | Rejki Mobile (pengguna umum) |
| **Dokumen** | Sub-PRD (bagian dari [rejki-prd.md](rejki-prd.md)) |
| **Versi** | 0.1 — Draft |
| **Tanggal** | 2026-06-10 |
| **Status** | Draft untuk ditinjau |
| **Pemilik** | _(belum ditentukan)_ |

> **Legenda status:** ✅ **Ada** (terverifikasi di kode) · 🔧 **Parsial** · 📋 **Rencana** (belum ada di backend).
> **Prioritas (MoSCoW):** M = Must, S = Should, C = Could, W = Won't (untuk sekarang).
> Requirement ber-status ✅ ditelusuri langsung ke handler/entity di `rust-services/`.

---

## 1. Ringkasan

Rejki Mobile adalah aplikasi **Flutter** (Dart) untuk **pengguna umum** (role user biasa). Pengguna dapat mendaftar, mengelola profil, memasang & menelusuri iklan pada empat vertikal (pekerjaan, pekerja, barang bekas, pelatihan), berkomunikasi via chat real-time, serta menerima notifikasi push. Sebagian besar kemampuan inti **sudah tersedia** di backend.

> Teknologi Flutter **tersurat** di dokumen standar Phase 1.5 ([api-standard.html](../api-standard.html), [websocket-contract.html](../websocket-contract.html), [security-baseline.html](../security-baseline.html)). Backend bersifat client-agnostic; tanggung jawab klien (mis. konversi waktu UTC→lokal) mengikuti kontrak API yang sudah ada.

## 2. Persona _(inferensi)_

> Disimpulkan dari domain; tidak tertulis eksplisit di repositori.

| Persona | Tujuan di Mobile |
|---|---|
| Pencari kerja | Menelusuri lowongan, menghubungi perekrut |
| Perekrut / Perusahaan | Memasang lowongan, menerima kontak kandidat |
| Freelancer / Penyedia jasa | Memasang profil keahlian & tarif |
| Klien / Pencari jasa | Menemukan pekerja sesuai keahlian & lokasi |
| Penjual barang bekas | Memasang barang, menandai terjual |
| Pembeli | Mencari barang, menghubungi penjual |
| Penyelenggara pelatihan | Mempublikasikan program & jadwal |
| Peserta pelatihan | Menemukan pelatihan yang relevan |

---

## 3. Requirement Fungsional

### 3.1 Autentikasi & Sesi

| ID | Requirement | Prioritas | Status |
|---|---|---|---|
| FR-MOB-AUTH-01 | Registrasi dengan email & password | M | ✅ Ada |
| FR-MOB-AUTH-02 | Verifikasi OTP saat registrasi | M | ✅ Ada |
| FR-MOB-AUTH-03 | Kirim ulang OTP | M | ✅ Ada |
| FR-MOB-AUTH-04 | Login menghasilkan access + refresh token (JWT RS256) | M | ✅ Ada |
| FR-MOB-AUTH-05 | Perpanjang sesi via refresh token (rotation) | M | ✅ Ada |
| FR-MOB-AUTH-06 | Logout (invalidasi refresh token) | M | ✅ Ada |
| FR-MOB-AUTH-07 | Lupa password / reset password | S | 📋 Rencana _(usulan)_ |

### 3.2 Profil Pengguna

| ID | Requirement | Prioritas | Status |
|---|---|---|---|
| FR-MOB-USR-01 | Melihat profil sendiri (`GET /users/me`) | M | ✅ Ada |
| FR-MOB-USR-02 | Mengubah profil sendiri (`PATCH /users/me`) | M | ✅ Ada |
| FR-MOB-USR-03 | Melihat profil pengguna lain (`GET /users/{id}`) | S | ✅ Ada |
| FR-MOB-USR-04 | Unggah/ubah foto profil (media) | S | 🔧 Parsial _(endpoint presigned URL `POST /users/me/avatar` ada & ter-wire ke StorageClient; upload nyata butuh env object storage MinIO/S3)_ |

### 3.3 Iklan Pekerjaan

| ID | Requirement | Prioritas | Status |
|---|---|---|---|
| FR-MOB-JOB-01 | Menelusuri daftar lowongan (`GET /pekerjaan`) | M | ✅ Ada |
| FR-MOB-JOB-02 | Melihat detail lowongan (`GET /pekerjaan/{id}`) | M | ✅ Ada |
| FR-MOB-JOB-03 | Memasang lowongan (`POST /pekerjaan`) | M | ✅ Ada |
| FR-MOB-JOB-04 | Menghapus lowongan milik sendiri (`DELETE`, IDOR→404) | M | ✅ Ada |
| FR-MOB-JOB-05 | Menyunting lowongan (update/PATCH) | S | 📋 Rencana _(usulan)_ |
| FR-MOB-JOB-06 | Filter berdasarkan tipe kerja / gaji / lokasi | S | 📋 Rencana _(usulan)_ |
| FR-MOB-JOB-07 | Melamar lowongan | C | 📋 Rencana _(usulan)_ |

### 3.4 Iklan Pekerja

| ID | Requirement | Prioritas | Status |
|---|---|---|---|
| FR-MOB-WRK-01 | Menelusuri daftar pekerja (`GET /pekerja`) | M | ✅ Ada |
| FR-MOB-WRK-02 | Melihat detail pekerja (`GET /pekerja/{id}`) | M | ✅ Ada |
| FR-MOB-WRK-03 | Memasang profil pekerja (`POST /pekerja`) | M | ✅ Ada |
| FR-MOB-WRK-04 | Menghapus profil pekerja milik sendiri (`DELETE`) | M | ✅ Ada |
| FR-MOB-WRK-05 | Pencarian berdasarkan keahlian (array `keahlian`) | S | 📋 Rencana _(usulan)_ |
| FR-MOB-WRK-06 | Menyunting profil pekerja | S | 📋 Rencana _(usulan)_ |

### 3.5 Iklan Barang Bekas

| ID | Requirement | Prioritas | Status |
|---|---|---|---|
| FR-MOB-GDS-01 | Menelusuri daftar barang (`GET /barang`) | M | ✅ Ada |
| FR-MOB-GDS-02 | Melihat detail barang (`GET /barang/{id}`) | M | ✅ Ada |
| FR-MOB-GDS-03 | Memasang barang (`POST /barang`) | M | ✅ Ada |
| FR-MOB-GDS-04 | Menghapus barang milik sendiri (`DELETE`) | M | ✅ Ada |
| FR-MOB-GDS-05 | Menandai barang terjual (`is_sold`) | M | 🔧 Parsial _(field ada di entity; endpoint khusus belum terverifikasi)_ |
| FR-MOB-GDS-06 | Unggah banyak foto (`foto_urls`) | M | 🔧 Parsial _(field ada; alur upload media belum)_ |
| FR-MOB-GDS-07 | Filter kondisi / harga / lokasi | S | 📋 Rencana _(usulan)_ |

### 3.6 Iklan Pelatihan

| ID | Requirement | Prioritas | Status |
|---|---|---|---|
| FR-MOB-TRN-01 | Menelusuri daftar pelatihan (`GET /pelatihan`) | M | ✅ Ada |
| FR-MOB-TRN-02 | Melihat detail pelatihan (`GET /pelatihan/{id}`) | M | ✅ Ada |
| FR-MOB-TRN-03 | Memasang pelatihan (`POST /pelatihan`) | M | ✅ Ada |
| FR-MOB-TRN-04 | Menghapus pelatihan milik sendiri (`DELETE`) | M | ✅ Ada |
| FR-MOB-TRN-05 | Filter berdasarkan tanggal / lokasi / harga | S | 📋 Rencana _(usulan)_ |
| FR-MOB-TRN-06 | Pendaftaran peserta pelatihan | C | 📋 Rencana _(usulan)_ |

### 3.7 Chat

| ID | Requirement | Prioritas | Status |
|---|---|---|---|
| FR-MOB-CHT-01 | Memulai/mengambil percakapan dengan pengguna lain | M | ✅ Ada |
| FR-MOB-CHT-02 | Mengirim pesan dalam percakapan | M | ✅ Ada |
| FR-MOB-CHT-03 | Memuat riwayat pesan (pagination) | M | ✅ Ada |
| FR-MOB-CHT-04 | Pesan real-time via WebSocket (auth saat handshake) | M | ✅ Ada |
| FR-MOB-CHT-05 | Indikator dibaca / typing | C | 📋 Rencana _(usulan)_ |

### 3.8 Notifikasi

| ID | Requirement | Prioritas | Status |
|---|---|---|---|
| FR-MOB-NOT-01 | Melihat daftar notifikasi in-app | M | ✅ Ada |
| FR-MOB-NOT-02 | Menandai notifikasi sebagai dibaca | M | ✅ Ada |
| FR-MOB-NOT-03 | Menerima push notification (FCM) | M | ✅ Ada _(jalur Redis Streams → Bun → FCM)_ |
| FR-MOB-NOT-04 | Registrasi/penghapusan device token | M | 🔧 Parsial _(konsumsi token oleh consumer ada; endpoint registrasi token belum terverifikasi)_ |

---

## 4. User Stories Utama

- **Pencari kerja:** _Sebagai pencari kerja, saya ingin menelusuri lowongan dan melihat detailnya, agar saya bisa menemukan pekerjaan yang sesuai._
- **Perekrut:** _Sebagai perekrut, saya ingin memasang lowongan dengan rentang gaji & tipe kerja, agar kandidat yang tepat tertarik._
- **Freelancer:** _Sebagai freelancer, saya ingin menampilkan keahlian & tarif saya, agar klien dapat menemukan saya._
- **Penjual:** _Sebagai penjual, saya ingin memasang barang bekas dengan foto & kondisi, lalu menandainya terjual setelah laku._
- **Penyelenggara pelatihan:** _Sebagai penyelenggara, saya ingin mempublikasikan pelatihan beserta jadwalnya, agar calon peserta dapat mendaftar._
- **Semua pengguna:** _Sebagai pengguna, saya ingin mengobrol langsung dengan pemasang iklan dan menerima notifikasi, agar transaksi lebih cepat._

---

## 5. Catatan Kesenjangan (Gap) untuk Phase Berikutnya

Fitur mobile umum yang **belum** ada di backend dan diusulkan untuk Phase 2+ _(usulan)_:

- **Pencarian & filter** lanjutan untuk keempat vertikal (termasuk pencarian keahlian via GIN index yang sudah disebut di konvensi DB).
- **Penyuntingan iklan** (update/PATCH) — saat ini hanya create & delete.
- **Manajemen media** — foto profil (avatar) sudah punya alur presigned URL via StorageClient (`POST /users/me/avatar`, ter-wire; butuh env object storage untuk upload nyata); upload foto iklan/barang (`foto_urls`) belum.
- **Registrasi device token** untuk push dari sisi aplikasi.
- **Pelaporan konten/pengguna** (report) — prasyarat untuk moderasi di [prd-dashboard.md](prd-dashboard.md).
- **Reset password** & pemulihan akun.

---

_Lihat juga: [rejki-prd.md](rejki-prd.md) (payung) · [prd-dashboard.md](prd-dashboard.md) · [prd-ceo.md](prd-ceo.md)._
