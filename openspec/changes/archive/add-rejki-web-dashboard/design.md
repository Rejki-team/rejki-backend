## Context

`rejki-web/` adalah direktori kosong (greenfield). Backend menyediakan REST/JSON dengan envelope `ApiResponse<T>` (`{ data, meta? }`), error RFC 9457 (`{ error: { type, title, status, detail } }`), dan prefix `/api/v1/`. Autentikasi: JWT RS256 via header `Authorization: Bearer <access_token>`. Refresh token rotation. Proposal ini mendefinisikan arsitektur klien SPA, pola komponen, dan kontrak konsumsi API — menjadi cetak biru implementasi Vue. Stack dikonfirmasi pemilik produk: Vue 3 + TypeScript + Tailwind. Acuan: seluruh halaman PRD Dashboard v0.2 §5.

## Goals / Non-Goals

**Goals:** Arsitektur SPA, auth flow, layout, pola UI lintas-halaman, kontrak konsumsi API per halaman, responsif.

**Non-Goals:** Kode Vue, backend mock, E2E test, PWA, i18n.

## Decisions

### D1 — Vue 3 Composition API + `<script setup>` + TypeScript strict
Mengikuti konvensi Vue 3 modern: Composition API dengan `<script setup lang="ts">`. TypeScript `strict: true`. Ini adalah rekomendasi resmi Vue 3 untuk proyek baru — lebih ringkas, tree-shaking lebih baik, dan type inference komposisi unggul.

### D2 — Pinia untuk state management
Pinia adalah state management resmi Vue 3 (pengganti Vuex). Store untuk: `auth` (token, user, role), `sidebar` (collapsed state), dan per-halaman bila diperlukan (cache data server-table). Tidak over-fetch — setiap halaman fetch data sendiri via composable.

### D3 — Axios dengan interceptor untuk auth
Axios instance tunggal dengan:
- **Request interceptor**: sisipkan `Authorization: Bearer <access_token>` dari auth store; cek token expiry sebelum kirim.
- **Response interceptor**: pada 401 → coba refresh token; jika gagal → redirect `/login`. Pada 403 → tampilkan toast "Akses ditolak".
- **Base URL**: `/api/v1` (di-serve oleh nginx reverse proxy yang sama saat produksi; development via Vite proxy).

### D4 — Composable `useServerTable` untuk tabel data
Abstraksi reusable untuk tabel server-side:
```ts
const { data, loading, error, pagination, search, sort, filters, refresh, exportCsv } = useServerTable<T>(endpoint, columns)
```
Props/return: `page`, `limit`, `total`, `q` (search), `status` (filter/sort), `rows`, skeleton loading, error state. Setiap halaman cukup memanggil composable ini dengan endpoint spesifik. Menghindari duplikasi logika fetch/paginasi/search/sort di 10+ halaman.

### D5 — Komponen `MaskedValue` + `ClickToView`
Untuk data sensitif (NIK, Foto KTP/Selfie):
- Default: tampil ter-mask (`xxxx...1234` untuk NIK, placeholder blur untuk gambar).
- Klik: panggil endpoint backend yang mencatat akses (audit `document_access_log` already exists), lalu tampilkan nilai penuh / URL presigned sementara.
- Setelah popup ditutup, kembali ke state ter-mask.
- Ini memenuhi FR-ADM-USR-02 (masking + click-to-view teraudit) dan praktik [PII data masking & audit logging](https://zuplo.com/learning-center/protect-sensitive-data-in-api-logs).

### D6 — Routing: Vue Router 4 dengan nested routes + guard
Struktur router:
```
/login                          → LoginPage
/ (auth guard + role=admin)     → DashboardLayout
  /pekerja                      → IklanPekerjaPage
  /pekerjaan                    → IklanPekerjaanPage
  /pelatihan                    → PelatihanLayout
    /pelatihan/daftar           → DaftarPelatihanPage
    /pelatihan/konfirmasi       → KonfirmasiPelatihanPage
    /pelatihan/badge            → BadgePelatihanPage
  /barang-bekas                 → BarangBekasPage
  /pengguna                     → PengelolaanPenggunaPage
  /dukungan                     → PengelolaanDukunganPage
  /corporate-communication      → CorporateCommunicationPage
```
Guard `beforeEnter` memeriksa auth store: tanpa token → redirect `/login`; `role ≠ admin` → toast + redirect.

### D7 — Tailwind CSS untuk styling
Tailwind CSS v4 dengan utility-first. Komponen UI mengikuti design system sederhana: warna consistent, spacing scale, radius, shadow. Komponen aksesibel (focus ring, ARIA label) mengandalkan Headless UI atau Radix Vue untuk komponen interaktif (modal, dropdown, popover) — opsional, dapat diganti native HTML + Tailwind.

### D8 — Loading & feedback global
- **Skeleton loader** (bukan spinner) untuk initial load tabel — lebih baik UX ([NNG table UX](https://www.eleken.co/blog-posts/table-design-ux)).
- **Toast** (notifikasi transient) untuk hasil aksi (berhasil/gagal) — posisi top-right, auto-dismiss. Komponen global di `App.vue`.
- **Modal konfirmasi** untuk aksi destruktif (suspend, hapus, tolak).

### D9 — Responsif: sidebar collapse + tabel adaptif
- **Sidebar**: di mobile penuh (overlay) via hamburger; di desktop sticky.
- **Tabel**: kolom lebar (deskripsi, alamat) di-wrap/sembunyi di mobile; prioritas kolom (ID, nama, status, aksi) tetap.
- **Kontrol pagination**: sederhana di mobile (Prev/Next, tanpa nomor halaman panjang).

## Risks / Trade-offs

- **Dependensi pada backend yang belum ada** — seluruh endpoint admin bergantung pada implementasi `add-admin-rbac` dkk. Development frontend dapat dimulai dengan mock API (MSW/json-server) setelah endpoint disepakati.
- **JWT decode di klien** — klien perlu mendekode payload JWT untuk membaca `role` tanpa kunci privat (validasi tetap di server). Gunakan library `jwt-decode` ringan.
- **CSV download** — endpoint backend mengembalikan `text/csv` stream; klien trigger download via anchor blob atau `fetch` → `Blob` → `URL.createObjectURL`.
- **Refresh token rotation** — implementasi di klien harus atomic (queue permintaan saat refresh berlangsung) agar tidak banyak request 401 bersamaan.

## Migration Plan

1. Scaffold proyek: `npm create vite@latest rejki-web -- --template vue-ts` + install dependencies (vue-router, pinia, axios, tailwindcss, jwt-decode).
2. Setup Tailwind, Vue Router, Pinia, dan axios instance.
3. Implementasi auth flow (login page, auth store, route guard, interceptor, header profil).
4. Implementasi layout (sidebar + menu routing).
5. Implementasi komponen UI lintas-halaman (server-table, masked-value, toast, modal, bulk-select, csv-export, skeleton).
6. Implementasi halaman per menu (satu per satu, sesuai prioritas).
7. Responsive testing & polish.

## Open Questions

- **Mock API untuk development**: MSW (Mock Service Worker) vs json-server vs Vite proxy ke backend lokal? Rekomendasi: Vite proxy ke backend lokal (development real) + fallback MSW untuk endpoint yang belum ada.
- **State persistence lintas sesi**: apakah token disimpan di `localStorage` (rentan XSS) atau `httpOnly` cookie? Rekomendasi: `httpOnly` cookie + CSRF token untuk produksi; `localStorage` acceptable untuk development. Backend perlu mendukung cookie-based auth — di luar scope proposal ini (backend saat ini pure Bearer header).
- **Komponen UI library**: Headless UI vs Radix Vue vs native? Rekomendasi: Headless UI (ringan, integrasi Tailwind baik, aksesibel).
