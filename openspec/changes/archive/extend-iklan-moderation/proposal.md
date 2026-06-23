## Why

Rejki Web Dashboard ([prd-dashboard.md](../../../docs/prd/prd-dashboard.md) §5.1/5.2/5.6) menuntut admin memoderasi keempat vertikal iklan: melihat daftar dalam tabel responsif dengan **foto popup**, **mencari** (judul/kode/pembuat), **mengurutkan by status**, **mengekspor CSV**, dan **men-suspend** iklan yang melanggar (single/sebagian/sekaligus) disertai **alasan + bukti** wajib serta notifikasi otomatis ke pemilik. Backend saat ini hanya menyediakan `GET /` (list `is_active`/`is_sold`), `GET /{id}`, `POST /`, `DELETE /{id}` (hard delete) dengan query `limit/offset` saja — **tanpa** state moderasi, soft-delete, search/sort, CSV, maupun endpoint admin. Selain itu beberapa field entity (`lokasi`, `gaji_*`/`tarif_*`/`harga`, `foto_urls`) **ada tetapi belum di-`INSERT`/di-surface** ke API. Proposal ini menambahkan kapabilitas moderasi admin untuk keempat vertikal, diproteksi RBAC (`add-admin-rbac`).

## What Changes

- **State moderasi iklan** (FR-ADM-*-06): tambah kolom `moderation_status` (`active`/`suspended_temp`/`suspended_permanent`) + `deleted_at` (soft-delete) pada keempat entity (`iklan_pekerjaan`, `iklan_pekerja`, `iklan_barang_bekas`, `iklan_pelatihan`). List publik hanya menampilkan iklan `active` & belum dihapus.
- **Suspend per-iklan** (FR-ADM-*-06): endpoint admin suspend single/bulk dengan `reason` wajib + **bukti** (gambar/PDF ≤5MB, 1 file) via presigned upload kategori baru `iklan-suspension-evidence`; simpan ke tabel `iklan_suspension`. Suspend memicu notifikasi **email + in-app** ke pemilik iklan via `NotificationClient`.
- **Listing admin dengan search/filter/sort/pagination** (FR-ADM-*-01/03/04): endpoint admin list per vertikal dengan pencarian (judul/kode/pembuat), filter & sort by `moderation_status`, pagination server-side (perluas `limit/offset`). Mengembalikan kolom sesuai User Story termasuk status.
- **Surface `foto_urls`** (FR-ADM-*-02): tambahkan/aktifkan field foto pada respons agar dashboard menampilkan **popup foto** per item. Untuk `iklan_barang_bekas`, `foto_urls` sudah ada sebagai kolom DB tetapi belum di-surface — kini di-plumbing ke create & response. Vertikal lain yang punya foto pekerjaan ditambahkan field setara.
- **Export CSV** (FR-ADM-*-05): endpoint admin `GET /admin/{vertikal}/export.csv` menghasilkan unduhan CSV daftar iklan (mengikuti filter aktif).
- **Perbaikan insert field** : pastikan field yang selama ini tak tersimpan (`lokasi`, `gaji_*`/`tarif_*`/`harga`, `tanggal_*`) ikut di-`INSERT` agar data lengkap untuk moderasi.
- **Admin read-only atas data iklan** (FR-ADM-*-08): admin **tidak** dapat menyunting data iklan; satu-satunya aksi mutasi admin adalah suspend.

## Capabilities

### New Capabilities
- `iklan-moderation-status`: state moderasi + soft-delete pada keempat entity iklan; pengaruhnya pada visibilitas list publik.
- `iklan-admin-listing`: listing admin per vertikal dengan search, filter/sort by status, pagination, dan kolom sesuai User Story (read-only).
- `iklan-suspension`: suspend per-iklan single/bulk dengan alasan + bukti wajib, penyimpanan riwayat suspensi, dan notifikasi ke pemilik.
- `iklan-media-visibility`: penyediaan foto (foto pekerjaan/barang) pada respons agar dapat ditampilkan sebagai popup.
- `iklan-csv-export`: ekspor daftar iklan ke CSV mengikuti filter aktif.
- `iklan-cooldown`: poster dengan suspend permanen tidak dapat membuat iklan baru selama 3 hari (cooldown).
- `iklan-moderation-enum`: `ModerationStatus` sebagai Rust enum untuk type safety — illegal state unrepresentable.
- `iklan-auto-expire`: auto-un-suspend untuk `suspended_temp` yang `expires_at`-nya sudah lewat (scheduler).
- `iklan-admin-single-query`: admin listing menggunakan `COUNT(*) OVER()` window function — 1 query, bukan 2.

### Modified Capabilities
<!-- Tidak ada spec ter-arsip di openspec/specs/. Iklan service belum punya spec arsip; seluruh kemampuan di atas didefinisikan sebagai kapabilitas baru. -->

## Impact

- **Kode**: keempat crate iklan (`iklan-pekerjaan-service`, `iklan-pekerja-service`, `iklan-barang-bekas-service`, `iklan-pelatihan-service`) — domain entity (tambah `moderation_status`, `deleted_at`, field foto), application service & DTO (search/sort/pagination, insert field yang kurang, suspend), infrastructure repository (query admin + soft-delete + suspension), interface handlers & router (sub-router `/admin/**` + export CSV); `rust-services/rejki-app/` (wire `require_admin`, `StorageClient`, `NotificationClient` ke router admin iklan).
- **Basis data** (schema per iklan): kolom `moderation_status TEXT NOT NULL DEFAULT 'active'` (+ CHECK) & `deleted_at TIMESTAMPTZ` pada keempat tabel; tabel `iklan_suspension` (id, iklan_id, vertikal, is_permanent, reason, evidence_object_key, expires_at, created_by, created_at) per schema (tanpa FK lintas-schema). Index pada `moderation_status`.
- **API**: endpoint admin baru `GET /api/v1/admin/{vertikal}` (list+search+sort), `POST /admin/{vertikal}/suspend` (+ `/suspend/evidence`), `GET /admin/{vertikal}/export.csv`; perluasan respons publik untuk menyertakan foto. Seluruh `/admin/**` diproteksi `require_admin`.
- **Dependensi**: `add-admin-rbac` (proteksi), `StorageClient` (presigned bukti, kategori `iklan-suspension-evidence`), `NotificationClient` (email + in-app).
- **Standar**: envelope `ApiResponse`, error RFC 9457-inspired, IDOR→404, soft-delete, propagasi `request_id`, snake_case DB. UX server-side pagination mengikuti [praktik tabel data](https://www.eleken.co/blog-posts/table-design-ux).

## Non-Goals

- Penyuntingan data iklan oleh admin (admin read-only kecuali suspend).
- Penyelarasan `lokasi` teks-bebas ke `region_id` (ditunda; konsumsi region-service di luar scope ini).
- Fitur pelatihan (verifikasi pengajuan, enrollment, badge) — dimiliki `add-pelatihan-enrollment-badge`; di sini pelatihan hanya memperoleh kolom moderasi dasar bila beririsan, detail lifecycle di change tersebut.
- Pelaporan/aduan pengguna — `add-content-reports`.
- Penyelesaian model "Barang Bekas Gratis" vs "jual-beli berharga" bila terjadi konflik domain — ditandai sebagai open question; entity saat ini punya `harga`.
