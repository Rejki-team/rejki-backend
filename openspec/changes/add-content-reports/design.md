## Context

Rejki belum memiliki domain pelaporan. Pola domain mandiri sudah mapan: `region-service` mendemonstrasikan pembuatan crate baru dengan 4 layer + client crate + wiring di `rejki-app`. `report` adalah domain baru serupa: skalanya kecil (satu tabel inti) sehingga setup cepat mengikuti pola. `StorageClient` sudah mendukung presigned upload; `NotificationClient` mendukung `send`/`send_bulk`/`send_email`. `add-admin-rbac` menyediakan `require_admin`. Pembuatan aduan dari sisi mobile adalah prasyarat (endpoint `POST /api/v1/reports` oleh pengguna), tetapi definisi endpoint tersebut ada di proposal ini agar kontrak API jelas. Acuan: FR-ADM-SUP-* di PRD Dashboard + FR-MOB-RPT-01 di PRD Mobile.

## Goals / Non-Goals

**Goals:** Domain report/aduan mandiri: create user + list/detail/review admin + bukti + notifikasi + CSV. Kepatuhan standar Phase 1.5.

**Non-Goals:** Aduan otomatis (AI), aduan chat, eskalasi multi-tier, UI dashboard.

## Decisions

### D1 — Crate baru `report-service` + `report-service-client`
Mengikuti pola `region-service`: domain baru sebagai crate mandiri. Skala kecil (satu tabel inti) tapi batas domain jelas — tidak menumpang di service iklan atau user-service. `report-service-client` mengekspos trait `ReportClient` bila domain lain perlu membaca status aduan (mis. iklan yang sedang diadukan).

### D2 — Satu tabel `report` dengan `target_type` diskriminator
`target_type TEXT CHECK (iklan, user)` menentukan apakah aduan terhadap iklan atau pengguna. `target_id` UUID referensial (tanpa FK lintas-schema — konsisten dengan pola). Bukti disimpan via `StorageClient` (kategori `report-evidence`, ≤5MB, JPEG/PNG/PDF). Status: `pending` → admin tinjau (`in_review`, opsional) → `resolved`/`rejected`.

### D3 — Admin wajib mengisi tindakan
Saat admin menyelesaikan aduan (`POST /admin/reports/{id}/review`), `action_note` wajib diisi — "tindakan yang telah dilakukan" (FR-ADM-SUP-02). Ini memastikan akuntabilitas. Reject juga memerlukan alasan (di `action_note`). Tombol Tolak/Terima terkunci setelah status ≠ `pending`/`in_review`.

### D4 — Notifikasi ke pihak terdampak
Approve/reject memicu `NotificationClient.send`/`send_email` ke pelapor dan (bila relevan) pihak yang diadukan. Kanal: in-app + email. Mengikuti pola notifikasi KYC di `add-user-service-kyc`.

### D5 — StorageClient kategori baru
`report-evidence` (≤5MB, JPEG/PNG/PDF) — bukti aduan dari pelapor. Alur unggah sama: request presigned → upload langsung → commit (magic bytes).

### D6 — Kepatuhan standar Phase 1.5
Envelope, error RFC 9457-inspired, propagasi `request_id`, snake_case DB.

## Risks / Trade-offs

- **Prasyarat mobile** — endpoint create report bergantung pada tim Flutter mengimplementasikan UI pelaporan.
- **Volume aduan** — kecil hingga moderat; paginasi sederhana cukup.
- **Notifikasi ke pihak diadukan** — perlu mengambil kontak (email) dari user-service/auth; pakai `AuthClient.get_account_email` (sudah ada) bila target = user; untuk iklan, ambil email pemilik via `poster_id`.

## Migration Plan

1. Buat crate `report-service` + `report-service-client`, daftarkan ke workspace.
2. Migrasi: schema `report` + tabel `report` + index.
3. Tambah kategori `report-evidence` di `StorageClient`.
4. Implementasi: endpoint create user (mobile), listing/detail/review admin, CSV.
5. Wire di `rejki-app` dengan `require_admin`, `StorageClient`, `NotificationClient`.
6. Rollback: migrasi turun drop schema `report`; tidak ada dampak lintas-schema.

## Open Questions

- Apakah aduan perlu dapat di-"reopen" bila pelapor tidak puas dengan tindak lanjut? Default: tidak (status terminal).
- Apakah perlu notifikasi ke admin saat aduan baru masuk? Default: tidak; admin memantau antrian.
