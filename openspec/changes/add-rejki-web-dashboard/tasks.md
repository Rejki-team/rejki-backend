## 1. Scaffold Proyek

- [ ] 1.1 Inisialisasi proyek Vite + Vue 3 + TypeScript di `rejki-web/`
- [ ] 1.2 Install dependencies: `vue-router@4`, `pinia`, `axios`, `tailwindcss`, `jwt-decode`, `@headlessui/vue` (opsional)
- [ ] 1.3 Install dev dependencies: `eslint`, `prettier`, `postcss`, `autoprefixer`
- [ ] 1.4 Konfigurasi Tailwind, TypeScript strict, Vite proxy ke backend (`/api` → `http://localhost:8080`)
- [ ] 1.5 Konfigurasi ESLint + Prettier

## 2. Autentikasi & Otorisasi Klien (spec: dashboard-auth)

- [ ] 2.1 Pinia `authStore`: state (`accessToken`, `refreshToken`, `user`, `role`), actions (`login`, `logout`, `refreshToken`, `fetchProfile`), getters (`isAuthenticated`, `isAdmin`)
- [ ] 2.2 Axios instance dengan request interceptor (attach Bearer) + response interceptor (refresh on 401, redirect /login on fail)
- [ ] 2.3 Halaman login (`/login`): form email+password → `POST /auth/admin/login`; error handling; redirect ke `/` setelah sukses
- [ ] 2.4 Vue Router guard `beforeEach`: tanpa token → `/login`; `role ≠ admin` → toast + redirect; token expired → refresh → retry
- [ ] 2.5 Header profil (foto + nama + role) + dropdown logout (panggil `POST /auth/logout` + clear store + redirect `/login`)

## 3. Layout & Navigasi (spec: dashboard-layout)

- [ ] 3.1 Layout dashboard: sidebar kiri + header atas + konten utama (router-view)
- [ ] 3.2 Sidebar menu sesuai User Story (Iklan Pekerja, Iklan Pekerjaan, Iklan Pelatihan dgn 3 sub-menu, Iklan Barang Bekas Gratis, Pengelolaan Pengguna, Pengelolaan Dukungan, Corporate Communication)
- [ ] 3.3 Sidebar toggle hamburger (buka/tutup); responsif (overlay di mobile, sticky di desktop)
- [ ] 3.4 Router nested routes untuk menu & sub-menu

## 4. Komponen UI Lintas-Halaman (spec: dashboard-patterns)

- [ ] 4.1 Composable `useServerTable<T>`: fetch data dengan pagination (`page`, `limit`), search (`q`), sort (`sort`, `order`), filter (`status`); kembalikan `{ data, loading, error, total, pagination, refresh, exportCsv }`
- [ ] 4.2 Komponen `ServerTable.vue`: render tabel + pagination + search input + dropdown filter/sort + slot untuk kolom kustom
- [ ] 4.3 Komponen `MaskedValue.vue`: default ter-mask (`***...`), klik → fetch niliai asli (catat audit) → tampilkan; tutup → mask kembali
- [ ] 4.4 Komponen `PopupFoto.vue`: modal/lightbox tampilkan foto dari URL presigned
- [ ] 4.5 Komponen `ToastContainer.vue` + `useToast()` composable: notifikasi transient global (success/error)
- [ ] 4.6 Komponen `ModalConfirm.vue`: dialog konfirmasi aksi (suspend/tolak) + form alasan + upload bukti
- [ ] 4.7 Komponen `BulkSelectCheckbox.vue`: pilih semua/sebagian + action bar "Suspend (N)"
- [ ] 4.8 Komponen `ExportCsvButton.vue`: tombol trigger download CSV dari endpoint admin
- [ ] 4.9 Komponen `SkeletonTable.vue`: skeleton loader saat data loading awal

## 5. Halaman: Iklan Pekerja (spec: dashboard-pages)

- [ ] 5.1 View `IklanPekerjaPage.vue`: tabel dengan kolom (ID, Nama, Pengalaman, Upah, Jam Kerja, Cara Hubungi, Foto Pekerjaan, Status) + search Nama/Kode/Pembuat + dropdown sort status
- [ ] 5.2 Popup foto pekerjaan per item via `PopupFoto.vue`
- [ ] 5.3 Bulk select + tombol Suspend → modal (pilih sementara/permanen + alasan + unggah bukti ≤5MB PDF/Gambar) → `POST /admin/pekerja/suspend`
- [ ] 5.4 Tombol Export CSV → `GET /admin/pekerja/export.csv`

## 6. Halaman: Iklan Pekerjaan (spec: dashboard-pages)

- [ ] 6.1 View `IklanPekerjaanPage.vue`: pola sama dengan Iklan Pekerja (kolom: ID, Judul, Deskripsi, Upah, Jam Kerja, Foto Pekerjaan, Status)
- [ ] 6.2 Suspend + Export CSV (sama pola)

## 7. Halaman: Iklan Pelatihan — Daftar Pelatihan (spec: dashboard-pages)

- [ ] 7.1 View `DaftarPelatihanPage.vue`: tabel (ID, Judul, Deskripsi, Penyelenggara, Lokasi, Tanggal, Jumlah Peserta, Status) + ikon Mata + tombol "Tambah Pelatihan"
- [ ] 7.2 Tombol "Tambah Pelatihan" → modal form (Judul, Deskripsi, Penyelenggara, Lokasi, Tanggal, Jumlah Peserta) → `POST /admin/pelatihan` (auto-approve)
- [ ] 7.3 Klik ikon Mata → popup detail + tombol kontekstual:
  - Jika `created_by_role=admin`: Batalkan / Edit (switch ke mode edit) / Simpan
  - Jika `created_by_role=user`: Tolak (wajib alasan) / Terima — terkunci setelah verifikasi
- [ ] 7.4 Edit mode: form field dapat disunting; tombol Simpan + Batalkan
- [ ] 7.5 Search (judul/kode/penyelenggara) + sort status + Export CSV

## 8. Halaman: Iklan Pelatihan — Konfirmasi Pelatihan (spec: dashboard-pages)

- [ ] 8.1 View `KonfirmasiPelatihanPage.vue`: tabel (ID Pelatihan, ID Pengguna, Judul, Penyelenggara, Tanggal, Foto Bukti Transfer, Status) + ikon Mata
- [ ] 8.2 Popup detail + tampilkan Foto Bukti Transfer + Tolak/Terima (alasan wajib, terkunci setelah verifikasi)
- [ ] 8.3 Search + sort + Export CSV

## 9. Halaman: Iklan Pelatihan — Badge Pelatihan (spec: dashboard-pages)

- [ ] 9.1 View `BadgePelatihanPage.vue`: tabel (ID Pelatihan, ID Pengguna, Nama Pengguna, Judul, Penyelenggara, Tanggal Badge, Sertifikat, Status) + ikon Mata
- [ ] 9.2 Popup detail + tampilkan Sertifikat + Tolak/Terima (alasan wajib, terkunci)
- [ ] 9.3 Search + sort + Export CSV

## 10. Halaman: Iklan Barang Bekas Gratis (spec: dashboard-pages)

- [ ] 10.1 View `BarangBekasPage.vue`: tabel (ID, Judul, Deskripsi, Jenis Barang, Foto Barang, Jumlah, Lokasi Pengambilan, Status) + popup foto
- [ ] 10.2 Suspend + Export CSV (pola sama)

## 11. Halaman: Pengelolaan Pengguna (spec: dashboard-pages)

- [ ] 11.1 View `PengelolaanPenggunaPage.vue`: tabel (ID, Nama, Pendidikan, Gender, TTL, Alamat, Wilayah, Status Verifikasi) + ikon Mata
- [ ] 11.2 Popup detail dengan `MaskedValue` untuk NIK + `ClickToView` untuk Foto KTP & Selfie (audit log akses)
- [ ] 11.3 Tolak/Terima (alasan wajib, terkunci setelah verifikasi) → `POST /admin/kyc/{id}/review`
- [ ] 11.4 Bulk select + Suspend pengguna (sementara/permanen + alasan + bukti) → `POST /admin/users/*/suspend`
- [ ] 11.5 Search (ID/Nama) + sort status + Export CSV

## 12. Halaman: Pengelolaan Dukungan (spec: dashboard-pages)

- [ ] 12.1 View `PengelolaanDukunganPage.vue`: tabel (ID Pengaduan, Tanggal, ID Pelapor, ID Iklan, Keterangan, Status) + ikon Mata
- [ ] 12.2 Popup detail + Foto Bukti + isi "tindakan yang dilakukan" (wajib) + Tolak/Terima → `POST /admin/reports/{id}/review`
- [ ] 12.3 Search (ID Pengguna/ID Aduan) + sort status + Export CSV

## 13. Halaman: Corporate Communication (spec: dashboard-pages)

- [ ] 13.1 View `CorporateCommunicationPage.vue`: tabel (ID, Pembuat [read-only], Kategori, Judul, Isi, Foto) + ikon Pena + ikon Tong Sampah
- [ ] 13.2 Tombol "Buat Artikel" → modal form (Judul, Isi, Kategori dropdown=informasi, Foto) → `POST /admin/articles`
- [ ] 13.3 Klik ikon Pena → popup lihat/edit (timestamps, pembuat read-only) + Batalkan/Simpan → `PUT /admin/articles/{id}`
- [ ] 13.4 Klik ikon Tong Sampah → konfirmasi hapus → `DELETE /admin/articles/{id}`
- [ ] 13.5 Search (judul) + sort kategori

## 14. Responsif & Polish

- [ ] 14.1 Sidebar overlay di mobile; tabel adaptif (kolom disederhanakan)
- [ ] 14.2 Paginasi adaptif mobile (Prev/Next sederhana, tanpa nomor halaman penuh)
- [ ] 14.3 Uji layar 375px–1920px untuk semua halaman

## 15. Finalisasi

- [ ] 15.1 ESLint + Prettier linting bersih
- [ ] 15.2 Build produksi (`vite build`) sukses tanpa error
- [ ] 15.3 Uji integrasi dengan backend lokal (Vite proxy)
- [ ] 15.4 Dokumentasi cara menjalankan (`README.md`) di `rejki-web/`
