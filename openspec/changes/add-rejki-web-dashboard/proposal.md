## Why

Rejki Web Dashboard ([prd-dashboard.md](../../../docs/prd/prd-dashboard.md) v0.2) adalah SPA **Vue.js 3 + TypeScript + Tailwind CSS** di repositori `rejki-web/` (saat ini kosong/greenfield). Seluruh backend admin telah dipetakan sebagai perubahan OpenSpec terpisah (`add-admin-rbac`, `extend-iklan-moderation`, `add-pelatihan-enrollment-badge`, `add-content-reports`, `add-corporate-comms`). Proposal ini adalah **spesifikasi UI/UX dan kontrak konsumsi API** untuk aplikasi web — tidak ada kode Rust.

## What Changes

- **Scaffold awal proyek**: Vite + Vue 3 (Composition API + `<script setup>`) + TypeScript (strict) + Tailwind CSS v4 + Vue Router 4 + Pinia (state management). Struktur folder konvensional (`src/views/`, `src/components/`, `src/stores/`, `src/router/`, `src/composables/`).
- **Autentikasi & otorisasi klien** (FR-ADM-AUTH-*): halaman login (email+password) → `POST /api/v1/auth/admin/login`; simpan access+refresh token di Pinia store; **intercept 401/403** → redirect login; **route guard** berdasarkan klaim `role=admin` (dari token JWT decode). Header menampilkan foto profil + nama + role (dari `GET /users/me`); menu logout → `POST /auth/logout`.
- **Layout & navigasi** (FR-ADM-NAV-*): sidebar collapsible (ikon hamburger) dengan menu persis User Story; sub-menu Iklan Pelatihan (3 sub); routing Vue Router per halaman.
- **Pola UI lintas-halaman** (FR-ADM-UX-*):
  - **Tabel server-side** (pagination, search, sort, filter) — data di-fetch per halaman; gunakan composable `useServerTable`. Loading skeleton/spinner.
  - **Popup foto** (modal/lighbox) untuk kolom foto.
  - **Masking + Click-to-View** untuk data sensitif (NIK, Foto KTP/Selfie) — terapkan komponen `MaskedValue` yang membuka penuh setelah klik + mencatat audit request ke backend.
  - **Toast/Modal status** (berhasil/gagal) global.
  - **Bulk select** (checkbox + actions) untuk suspend massal.
  - **Export CSV** — tombol trigger download dari endpoint backend.
- **Halaman per menu** (FR-ADM-*): satu view Vue per menu/sbu-menu, merender tabel dengan kolom sesuai User Story, menyediakan form/aksi sesuai spec (pop-up detail, Tolak/Terima, Suspend, Tambah/Edit, dll.). Seluruh aksi memanggil endpoint `/api/v1/admin/**` dengan bearer token.
- **Error handling konsisten**: tangani envelope `ApiResponse<T>`, error RFC 9457, IDOR (404), jaringan/timeout.
- **Responsif**: layout adaptif (sidebar toggle mobile, tabel horizontal-scroll/simplified columns).

## Capabilities

### New Capabilities
- `dashboard-auth`: login admin UI, token store, route guard, profil header + logout.
- `dashboard-layout`: sidebar navigasi collapsible, menu/sub-menu sesuai User Story.
- `dashboard-patterns`: komponen reusable (server-table, masked-value, popup-foto, toast, bulk-select, csv-export, loading-skeleton, form-modal).
- `dashboard-pages`: seluruh halaman per menu (Iklan Pekerja, Iklan Pekerjaan, Daftar/Konfirmasi/Badge Pelatihan, Iklan Barang Bekas Gratis, Pengelolaan Pengguna, Pengelolaan Dukungan, Corporate Communication) — lihat spesifikasi turunan.

## Impact

- **Repositori**: `rejki-web/` (sibling dari `rejki-backend/`) — saat ini kosong; menjadi root proyek Vue.
- **Dependensi runtime**: `axios` (HTTP client), `vue`, `vue-router`, `pinia`, `tailwindcss`, `@headlessui/vue` atau `@radix-ui/vue` (komponen aksesibel opsional).
- **Dependensi dev**: `vite`, `typescript`, `eslint`, `prettier`.
- **API endpoint admin yang dikonsumsi**:
  - `POST /api/v1/auth/admin/login` (login admin)
  - `POST /api/v1/auth/logout`
  - `GET /api/v1/users/me` (profil admin + role)
  - Seluruh `/api/v1/admin/**` dari keempat change backend (`extend-iklan-moderation`, `add-pelatihan-enrollment-badge`, `add-content-reports`, `add-corporate-comms`)
  - Admin review KYC & dokumen dari `add-user-service-kyc` (`POST /admin/kyc/{id}/review`, `GET /me/documents/{kind}`)
  - Suspend pengguna dari `extend-auth-service-onboarding` (`/admin/users/*`)
- **Tidak ada perubahan backend** — proposal ini murni UI/UX & kontrak konsumsi.

## Non-Goals

- Implementasi kode Vue (proposal ini adalah spesifikasi; implementasi setelah proposal disetujui).
- Backend mock / MSW (mock service worker) untuk development offline — di luar scope.
- E2E testing (Cypress/Playwright) — di luar scope iterasi awal.
- PWA/offline support.
- Internationalization (i18n) — bahasa Indonesia saja.
