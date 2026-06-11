# PRD — Rejki CEO Mobile

| | |
|---|---|
| **Aplikasi** | Rejki CEO Mobile (statistik eksekutif) |
| **Dokumen** | Sub-PRD (bagian dari [rejki-prd.md](rejki-prd.md)) |
| **Versi** | 0.1 — Draft |
| **Tanggal** | 2026-06-10 |
| **Status** | Draft untuk ditinjau |
| **Pemilik** | _(belum ditentukan)_ |

> ⚠️ **DISCLAIMER PENTING.** Seluruh isi berstatus **📋 Rencana** dan merupakan **_(USULAN)_** berbasis praktik standar industri untuk marketplace. Backend saat ini **belum** memiliki layer agregasi/analytics apa pun. Metrik di bawah adalah usulan untuk dikoreksi — **tidak ada angka/target yang dikarang**, hanya definisi & sumber data.

---

## 1. Ringkasan & Tujuan

Rejki CEO Mobile adalah aplikasi **Flutter** (Dart) yang bersifat **read-only** _(usulan default)_ untuk eksekutif memantau kesehatan bisnis platform dan mengidentifikasi peluang ekspansi. Penekanan khusus pada **sebaran geografis** sebagai acuan **canvassing** (menentukan wilayah prioritas untuk akuisisi pengguna/penjual).

> Catatan: stack Flutter dikonfirmasi pemilik produk (berbagi teknologi dengan Rejki Mobile). Seluruh kemampuan backend untuk aplikasi ini tetap berstatus 📋 Rencana.

**Tujuan:**
- Memberi gambaran cepat kondisi bisnis (pertumbuhan, likuiditas pasar, engagement).
- Menyoroti wilayah dengan potensi/kesenjangan suplai–permintaan.
- Menjadi acuan pengambilan keputusan ekspansi & canvassing.

---

## 2. Karakteristik Pengguna _(inferensi)_

| Persona | Kebutuhan |
|---|---|
| CEO / Eksekutif | Ringkasan tingkat tinggi, tren, sinyal wilayah; konsumsi cepat di mobile |

---

## 3. Mode Interaksi _(USULAN)_

- **Default read-only**: melihat dashboard & detail metrik.
- **Aksi ringan** _(usulan)_: filter periode (harian/mingguan/bulanan), filter wilayah, ekspor/berbagi ringkasan (PDF/gambar).
- Tidak ada aksi operasional (moderasi/manajemen) — itu ranah [Web Dashboard](prd-dashboard.md).

---

## 4. Metrik _(semua 📋 Rencana / USULAN)_

> Kolom **Sumber data** merujuk entity/tabel yang **sudah ada** sehingga metrik ini *layak* dihitung begitu layer agregasi dibangun. Lihat entity di [rejki-prd.md › Lampiran B](rejki-prd.md).

### 4.1 Pertumbuhan Pengguna

| ID | Metrik | Definisi | Sumber data |
|---|---|---|---|
| M-CEO-GRW-01 | Pengguna baru | Jumlah registrasi per periode (harian/mingguan/bulanan) | `auth.users.created_at` |
| M-CEO-GRW-02 | Total pengguna | Akumulasi pengguna terdaftar | `auth.users` |
| M-CEO-GRW-03 | Pengguna terverifikasi | Proporsi `is_verified = true` | `auth.users.is_verified` |
| M-CEO-GRW-04 | Retensi / pengguna aktif | Pengguna dengan aktivitas pada periode | _(perlu definisi "aktif" — **TBD**)_ |

### 4.2 Konten & Likuiditas Pasar

| ID | Metrik | Definisi | Sumber data |
|---|---|---|---|
| M-CEO-CNT-01 | Iklan aktif per vertikal | Jumlah iklan `is_active`/belum `is_sold` per kategori | entity 4 vertikal |
| M-CEO-CNT-02 | Iklan baru per periode | Iklan dibuat per periode per vertikal | `created_at` tiap entity iklan |
| M-CEO-CNT-03 | Rasio barang terjual | `is_sold = true` / total barang | `IklanBarangBekas.is_sold` |
| M-CEO-CNT-04 | Komposisi tipe pekerjaan | Distribusi `tipe` (full_time/part_time/freelance/internship) | `IklanPekerjaan.tipe` |
| M-CEO-CNT-05 | Sebaran keahlian populer | Frekuensi nilai pada array `keahlian` | `IklanPekerja.keahlian` |

### 4.3 Geografis (Acuan Canvassing)

| ID | Metrik | Definisi | Sumber data |
|---|---|---|---|
| M-CEO-GEO-01 | Sebaran iklan per wilayah | Jumlah iklan per `lokasi`/kota | field `lokasi` tiap entity |
| M-CEO-GEO-02 | Sebaran pengguna per wilayah | Distribusi pengguna per wilayah | _(perlu lokasi pengguna — **TBD**, mungkin via profil)_ |
| M-CEO-GEO-03 | Kesenjangan suplai–permintaan | Wilayah dengan banyak pencari tetapi sedikit penyedia (atau sebaliknya) | turunan M-CEO-GEO-01/02 |
| M-CEO-GEO-04 | Wilayah prioritas canvassing | Peringkat wilayah berdasar potensi pertumbuhan | turunan _(usulan komposit)_ |

> Catatan: `lokasi` saat ini bertipe teks bebas (opsional). Untuk analitik wilayah yang andal, disarankan **normalisasi wilayah/kota** (lihat [prd-dashboard.md › FR-ADM-CAT-02](prd-dashboard.md)).

### 4.4 Engagement

| ID | Metrik | Definisi | Sumber data |
|---|---|---|---|
| M-CEO-ENG-01 | Pesan chat terkirim | Volume pesan per periode | domain chat |
| M-CEO-ENG-02 | Percakapan aktif | Jumlah percakapan dengan aktivitas | domain chat |
| M-CEO-ENG-03 | Notifikasi terkirim | Volume notifikasi keluar | domain notification |
| M-CEO-ENG-04 | Notifikasi dibaca | Rasio dibaca/terkirim | status baca notifikasi |

### 4.5 Tren Waktu

| ID | Metrik | Definisi |
|---|---|---|
| M-CEO-TRD-01 | Time-series semua metrik inti | Grafik per hari/minggu/bulan |
| M-CEO-TRD-02 | Perbandingan periode | Pertumbuhan WoW / MoM |

---

## 5. Requirement Fungsional Aplikasi _(USULAN)_

| ID | Requirement | Prioritas |
|---|---|---|
| FR-CEO-01 | Dashboard ringkas KPI utama saat dibuka | M |
| FR-CEO-02 | Drill-down per metrik (lihat detail & tren) | M |
| FR-CEO-03 | Filter periode (harian/mingguan/bulanan) | M |
| FR-CEO-04 | Filter & peta/peringkat wilayah (canvassing) | M |
| FR-CEO-05 | Ekspor/berbagi ringkasan (PDF/gambar) | S |
| FR-CEO-06 | Perbandingan periode (WoW/MoM) | S |

---

## 6. Requirement Non-Fungsional _(USULAN)_

- **Akses sangat terbatas** — hanya peran eksekutif (RBAC; lihat [prd-dashboard.md](prd-dashboard.md)).
- **Read-only & aman** — tanpa kemampuan mengubah data operasional.
- **Kinerja** — agregasi tidak boleh membebani jalur transaksional; gunakan replika baca / materialized view / pra-agregasi _(usulan)_.
- **Kerahasiaan** — data agregat bersifat sensitif bisnis; enkripsi in-transit (sudah ada via edge) & kontrol akses ketat.

---

## 7. Kebutuhan Backend Baru (Tersirat) _(Rencana)_

1. **Layer analytics/reporting** read-only: endpoint agregasi (mis. `/api/v1/insights/...`) dengan proteksi peran eksekutif.
2. **Strategi agregasi**: materialized view / tabel ringkasan terjadwal, atau service statistik terpisah, agar tidak membebani DB transaksional.
3. **Normalisasi wilayah** (kota/provinsi) agar metrik geografis akurat.
4. **Definisi metrik bisnis** yang disepakati (terutama "pengguna aktif", "retensi", "wilayah prioritas").

---

## 8. Pertanyaan Terbuka

- Apakah akan ada **transaksi/pembayaran** internal? (menentukan metrik nilai transaksi/GMV yang kini **tidak** bisa dihitung)
- Definisi resmi "pengguna aktif" & jendela retensi.
- Granularitas wilayah yang diinginkan (kota? kecamatan? provinsi?).
- Apakah CEO perlu menetapkan target/anotasi (mengubah sifat read-only)?

---

_Lihat juga: [rejki-prd.md](rejki-prd.md) (payung) · [prd-mobile.md](prd-mobile.md) · [prd-dashboard.md](prd-dashboard.md)._
