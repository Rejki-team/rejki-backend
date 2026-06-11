# PRD — Rejki Web Dashboard (Admin)

| | |
|---|---|
| **Aplikasi** | Rejki Web Dashboard (admin & moderasi) |
| **Dokumen** | Sub-PRD (bagian dari [rejki-prd.md](rejki-prd.md)) |
| **Versi** | 0.1 — Draft |
| **Tanggal** | 2026-06-10 |
| **Status** | Draft untuk ditinjau |
| **Pemilik** | _(belum ditentukan)_ |

> ⚠️ **DISCLAIMER PENTING.** Seluruh isi dokumen ini berstatus **📋 Rencana** dan merupakan **_(USULAN)_** berbasis praktik standar industri. **Tidak satu pun** fitur di sini sudah ada di backend saat ini: kode tidak memiliki konsep peran/RBAC, endpoint admin, state moderasi, maupun pelaporan. Dokumen ini **disengaja dibuat detail agar mudah Anda koreksi** — bukan keputusan final.

---

## 1. Ringkasan & Tujuan

Rejki Web Dashboard adalah aplikasi web internal (**Vue.js + TypeScript + Tailwind CSS**) untuk **admin & moderator** menjaga kualitas konten dan mengelola operasional platform: meninjau & memoderasi iklan, menindaklanjuti laporan, mengelola pengguna, menyiarkan pengumuman, dan menelusuri jejak audit.

> Catatan: stack frontend (Vue + TS + Tailwind) dikonfirmasi pemilik produk. **Backend** untuk dashboard (RBAC, endpoint admin) tetap berstatus 📋 Rencana sebagaimana seluruh dokumen ini.

**Tujuan:**
- Menjaga kualitas & keamanan konten lintas empat vertikal.
- Memberi operator alat untuk menindak pelanggaran secara cepat & terdokumentasi.
- Menyediakan jejak audit yang akuntabel atas setiap tindakan admin.

---

## 2. Peran & Model Akses _(USULAN)_

Membutuhkan penambahan **RBAC** di backend (kolom `role` / tabel peran + middleware otorisasi peran).

| Peran | Cakupan akses _(usulan)_ |
|---|---|
| **Super Admin** | Akses penuh, termasuk manajemen admin & konfigurasi platform |
| **Moderator** | Moderasi konten, tindak laporan, suspend pengguna |
| **Support** | Baca data pengguna, bantu penyelesaian tiket; tanpa aksi destruktif |

Prinsip: **least-privilege**, setiap aksi sensitif tercatat di audit log.

---

## 3. Requirement Fungsional _(semua 📋 Rencana / USULAN)_

> **Prioritas (MoSCoW):** M/S/C/W.

### 3.1 Dashboard Operasional

| ID | Requirement | Prioritas |
|---|---|---|
| FR-ADM-DSH-01 | Ringkasan KPI operasional (antrian moderasi, laporan terbuka, pengguna baru) | M |
| FR-ADM-DSH-02 | Pintasan ke item yang butuh tindakan | S |

### 3.2 Moderasi Iklan (4 vertikal)

| ID | Requirement | Prioritas |
|---|---|---|
| FR-ADM-MOD-01 | Antrian moderasi iklan baru/terlapor (pekerjaan, pekerja, barang, pelatihan) | M |
| FR-ADM-MOD-02 | Aksi: approve / reject / takedown dengan alasan wajib | M |
| FR-ADM-MOD-03 | State moderasi pada iklan (`pending`/`approved`/`rejected`/`taken_down`) | M |
| FR-ADM-MOD-04 | Riwayat moderasi per iklan | S |
| FR-ADM-MOD-05 | Pencarian & filter iklan (status, kategori, pelapor, wilayah) | S |

> Implikasi backend: menambah kolom **status moderasi** + **soft-delete** pada entity iklan, serta endpoint admin.

### 3.3 Manajemen Pengguna

| ID | Requirement | Prioritas |
|---|---|---|
| FR-ADM-USR-01 | Cari & lihat detail pengguna | M |
| FR-ADM-USR-02 | Verifikasi / batalkan verifikasi pengguna | S |
| FR-ADM-USR-03 | Suspend / banned sementara atau permanen (dengan alasan & durasi) | M |
| FR-ADM-USR-04 | Reset/blokir sesi pengguna (force logout) | S |
| FR-ADM-USR-05 | Lihat aktivitas pengguna (iklan, laporan terkait) | S |

### 3.4 Laporan (Report) Konten & Pengguna

| ID | Requirement | Prioritas |
|---|---|---|
| FR-ADM-REP-01 | Daftar laporan masuk (alasan, pelapor, objek terlapor) | M |
| FR-ADM-REP-02 | Tindak lanjut laporan → tautkan ke aksi moderasi/penindakan | M |
| FR-ADM-REP-03 | Status laporan (`open`/`in_review`/`resolved`/`dismissed`) | M |

> Implikasi backend: fitur **pelaporan** di sisi Mobile (lihat [prd-mobile.md](prd-mobile.md) §5) + penyimpanan laporan.

### 3.5 Master Data / Kategori

| ID | Requirement | Prioritas |
|---|---|---|
| FR-ADM-CAT-01 | Kelola nilai master (mis. tipe pekerjaan, kondisi barang) | S |
| FR-ADM-CAT-02 | Kelola daftar wilayah/kota (mendukung canvassing & filter) | S |

### 3.6 Broadcast / Notifikasi Massal

| ID | Requirement | Prioritas |
|---|---|---|
| FR-ADM-BRD-01 | Kirim pengumuman/notifikasi ke segmen pengguna | S |
| FR-ADM-BRD-02 | Penjadwalan & riwayat broadcast | C |

> Memanfaatkan jalur notifikasi yang sudah ada (Redis Streams → FCM) dengan endpoint admin baru.

### 3.7 Moderasi Chat / Abuse _(opsional)_

| ID | Requirement | Prioritas |
|---|---|---|
| FR-ADM-CHT-01 | Tinjau percakapan yang dilaporkan | C |
| FR-ADM-CHT-02 | Tindak penyalahgunaan (peringatan/blokir) | C |

### 3.8 Audit Log

| ID | Requirement | Prioritas |
|---|---|---|
| FR-ADM-AUD-01 | Catat setiap aksi admin (aktor, objek, waktu, alasan) | M |
| FR-ADM-AUD-02 | Telusuri & filter audit log | S |

### 3.9 Manajemen Admin & Peran _(Super Admin)_

| ID | Requirement | Prioritas |
|---|---|---|
| FR-ADM-ADM-01 | Tambah/nonaktifkan akun admin | M |
| FR-ADM-ADM-02 | Tetapkan peran (Super Admin/Moderator/Support) | M |

---

## 4. Requirement Non-Fungsional _(USULAN)_

- **RBAC ketat** — otorisasi per peran di setiap endpoint admin; default deny.
- **Audit trail** — semua aksi sensitif tercatat & tak dapat diubah.
- **Sesi admin** — masa aktif lebih pendek, dukungan force-logout, idealnya MFA _(usulan)_.
- **Keamanan** — mengikuti baseline yang sudah ada ([security-baseline.html](../security-baseline.html)).
- **Observability** — aksi admin masuk logging terstruktur dengan `request_id`.

---

## 5. Kebutuhan Backend Baru (Tersirat) _(Rencana)_

1. **RBAC**: kolom `role` pada identitas / tabel peran + middleware otorisasi peran.
2. **Endpoint admin** terpisah (mis. namespace `/api/v1/admin/...`) dengan proteksi peran.
3. **State moderasi & soft-delete** pada entity iklan.
4. **Domain pelaporan** (report) untuk konten & pengguna.
5. **Penindakan pengguna** (suspend/ban) + pengecekan status saat auth.
6. **Audit log** persisten.

---

_Lihat juga: [rejki-prd.md](rejki-prd.md) (payung) · [prd-mobile.md](prd-mobile.md) · [prd-ceo.md](prd-ceo.md)._
