## Why

Rejki Web Dashboard ([prd-dashboard.md](../../../docs/prd/prd-dashboard.md) §5.3–§5.5) menuntut pengelolaan **siklus pelatihan** yang jauh melampaui entity saat ini: tujuh status pelatihan, **auto-approve** untuk pelatihan yang dibuat admin vs **review** (approve/reject) untuk yang diajukan pengguna, **Konfirmasi Pelatihan** (pendaftaran peserta + bukti transfer yang diverifikasi admin), dan **Badge Pelatihan** (sertifikat yang diverifikasi admin). Backend `iklan-pelatihan-service` saat ini hanya punya entity dasar (`judul`, `penyelenggara`, `deskripsi`, `lokasi?`, `harga?`, `tanggal_*?`, `is_active`) dengan route create/list/get/delete; **tidak ada** konsep status moderasi, peserta/enrollment, bukti transfer, badge, maupun sertifikat (penelusuran kode 2026-06-14: nol kecocokan untuk `enroll`/`peserta`/`badge`/`sertifikat`/`transfer`). Selain itu field `lokasi`/`harga`/`tanggal_*` ada di DTO tetapi **tidak ikut di-INSERT**. Proposal ini menambahkan domain enrollment & badge serta status moderasi pelatihan, diproteksi RBAC (`add-admin-rbac`). Pendaftaran & pengajuan badge dibuat dari sisi mobile (prasyarat, lihat [prd-mobile.md](../../../docs/prd/prd-mobile.md) §5.1).

## What Changes

- **Status moderasi pelatihan (7 status)** (FR-ADM-TRN-02): `verifikasi_tertunda`, `verifikasi_dalam_proses`, `verifikasi_ditolak`, `verifikasi_diterima`, `pelatihan_belum_dimulai`, `pelatihan_berjalan`, `pelatihan_selesai`. Tambah `created_by_role` (admin vs user) dan `jumlah_peserta`.
- **Auto-approve admin vs review user** (FR-ADM-TRN-03/04/06): pelatihan yang dibuat admin langsung `verifikasi_diterima` (auto-approve); pelatihan yang diajukan user mulai `verifikasi_tertunda` dan menunggu review admin (approve/reject + alasan). Admin **boleh edit/cancel** pelatihan miliknya; **tidak boleh edit** milik user (hanya Tolak/Terima). Tombol Tolak/Terima **terkunci** setelah verifikasi.
- **Konfirmasi Pelatihan / Enrollment** (FR-ADM-CNF-01..05): tabel `pelatihan_enrollment` (peserta mendaftar + `bukti_transfer_object_key`) dengan status (`pending`/`in_review`/`rejected`/`approved`); admin meninjau (approve/reject + alasan wajib bila tolak) + notifikasi otomatis. Tak dapat disunting.
- **Badge / Sertifikat** (FR-ADM-BDG-01..04): tabel `pelatihan_badge` (`sertifikat_object_key`, `approved_at`, status verifikasi admin); admin meninjau (approve/reject + alasan) + notifikasi. Tak dapat disunting.
- **Listing admin** untuk ketiga sub-halaman: search (judul/kode/penyelenggara), filter/sort by status, export CSV.
- **Perbaikan insert field** : `lokasi`, `harga`, `tanggal_mulai`, `tanggal_selesai` ikut tersimpan.

## Capabilities

### New Capabilities
- `pelatihan-moderation`: tujuh status pelatihan, `created_by_role`, auto-approve admin vs review user (approve/reject + alasan), aturan edit/cancel admin-only, dan penguncian aksi pasca-verifikasi.
- `pelatihan-enrollment`: pendaftaran peserta dengan bukti transfer, review admin (approve/reject), dan notifikasi hasil.
- `pelatihan-badge`: pengajuan badge/sertifikat, review admin (approve/reject), penyimpanan `approved_at`, dan notifikasi hasil.
- `pelatihan-admin-listing`: listing admin ketiga sub-halaman dengan search/filter/sort dan export CSV.

### Modified Capabilities
<!-- Tidak ada spec ter-arsip di openspec/specs/. Pelatihan belum punya spec arsip; kemampuan di atas didefinisikan sebagai kapabilitas baru. -->

## Impact

- **Kode**: `rust-services/iklan-pelatihan-service/` — domain (enum 7 nilai + `status_name`/`role_name`/`enrollment_status_name` named constants, `created_by_role`, `jumlah_peserta`, entity enrollment & badge), application service & DTO (create admin auto-approve, review submission, enroll + review, badge + review, search/sort, extracted helpers), infrastructure repository (tabel baru + query admin, `CreatePelatihanParams`/`UpdatePelatihanParams` structs), interface handlers & router (sub-router `/admin/**` + endpoint enroll/badge sisi user); `iklan-pelatihan-service-client/` (perluasan bila perlu); `rust-services/rejki-app/` (wire `require_admin`, `StorageClient`, `NotificationClient`). Pattern params struct + named constants diterapkan ke `iklan-pekerja-service`, `iklan-barang-bekas-service`, `iklan-pekerjaan-service`.
- **Basis data** (schema pelatihan): perluasan tabel pelatihan (`status` 7 nilai + CHECK, `created_by_role`, `jumlah_peserta`); tabel `pelatihan_enrollment` (id, pelatihan_id, user_id, bukti_transfer_object_key, status, reviewed_by, review_note, timestamps); tabel `pelatihan_badge` (id, pelatihan_id, user_id, sertifikat_object_key, approved_at, status, reviewed_by, review_note, timestamps). Index pada `status`. Tanpa FK lintas-schema.
- **API**: endpoint admin `GET/POST /api/v1/admin/pelatihan*` (daftar/verifikasi/edit/cancel), `GET/POST /admin/enrollments*`, `GET/POST /admin/badges*`, export CSV; endpoint sisi user `POST /pelatihan/{id}/enroll` (+ bukti) & `POST /pelatihan/{id}/badge` (prasyarat mobile).
- **Dependensi**: `add-admin-rbac` (proteksi), `StorageClient` (bukti transfer & sertifikat — kategori baru), `NotificationClient` (email + in-app). Lintas-aplikasi: pendaftaran & pengajuan badge dari Rejki Mobile.
- **Standar**: envelope `ApiResponse`, error RFC 9457-inspired, IDOR→404, propagasi `request_id`, snake_case DB, `created_at`/`updated_at`.

## Non-Goals

- Pembayaran/escrow internal (hanya unggah bukti transfer manual untuk diverifikasi admin; tidak ada pemrosesan pembayaran).
- Penerbitan sertifikat otomatis oleh sistem (sertifikat diunggah/diajukan, lalu diverifikasi admin).
- Penyelarasan `lokasi` pelatihan ke `region_id` (ditunda).
- Moderasi/suspend pelatihan sebagai "iklan" lintas-vertikal — kolom moderasi dasar dikoordinasikan dengan `extend-iklan-moderation` agar tidak duplikat; lifecycle 7-status dimiliki proposal ini.
- UI dashboard (komponen Vue) — `add-rejki-web-dashboard`.
