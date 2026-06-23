# Proposal: ws-housekeeping

> **Phase 3 — Workstream 1 (Housekeeping)**
> **Depends on:** — (tidak ada)
> **Blocks:** `ws-testing-coverage` (P2), dan seluruh workstream hilir Phase 3.

## Why

Phase 1, 1.5, dan 2 telah dideklarasikan selesai di `docs/index.html`, namun terdapat **tiga
masalah housekeeping** yang harus diselesaikan sebelum Phase 3 hardening dimulai, karena ketiganya
mengaburkan kebenaran status dan menyandera prioritas:

1. **Checklist Phase 3 menyesatkan.** Master Checklist `docs/index.html` menandai beberapa item
   Phase 3 sebagai "Pending" padahal kenyataannya sudah didokumentasikan selesai di Phase 1.5 dan
   sudah terimplementasi di kode: structured JSON logging + `request_id` per request
   (`common/tracing-setup/src/lib.rs`), multi-stage Dockerfile, GitHub Actions
   (`.github/workflows/{ci,deploy,deploy-dev}.yml`), `nginx.{dev,prod}.conf`, dan Cloudflare Tunnel.
   Label "Pending" yang keliru ini membuat scope Phase 3 terlihat lebih besar dari kenyataan dan
   menyembunyikan gap yang benar-benar belum ada.

2. **Dua OpenSpec change siap-diarsipkan tetapi belum diarsipkan.** `add-storage-service-spec`
   (semua task `[x]`, lolos clippy) dan `fix-phase2-code-review-findings` (45/46 task `[x]`, sisa
   hanya "archive change ini") sudah selesai tetapi masih menempati direktori aktif. Ini menambah
   noise pada daftar change aktif dan menyulitkan pelacakan pekerjaan tersisa.

3. **Sisa task test/coverage di `fix-auth-service-code-review-findings` dan
   `fix-user-service-code-review-findings` bersifat deferred-by-design** — task-task tersebut
   secara eksplisit menundakan pekerjaan coverage ke CI / integration test di `rejki-app`.
   Menyelesaikan task coverage tersebut di sini akan menduplikasi pekerjaan yang menjadi inti
   `ws-testing-coverage` (P2). Task ini harus ditandai secara akurat (bukan `[x]` palsu) agar
   P2 dapat mengambil alih tanpa ambiguitas kepemilikan.

Tujuan workstream ini: merapikan kebenaran status (checklist + arsip) dan menutup dua change
siap-arsip, sehingga Phase 3 dimulai dari kondisi yang jujur dan tanpa tumpang-tindih kepemilikan
dengan P2.

## What Changes

- **Perbaiki Master Checklist `docs/index.html`**: tandai item yang sudah terimplementasi sebagai
  "✓ Selesai" (structured logging + request_id; multi-stage Dockerfile; GitHub Actions fmt→clippy→
  test→build→push; nginx.dev/prod lengkap dengan WebSocket proxy + rate limit; Cloudflare Tunnel
  TUNNEL_TOKEN). Pertahankan "Pending" hanya untuk item yang benar-benar belum: rate limiting
  global (tower-governor) di luar auth-service, integration test per-service dengan DB nyata,
  coverage gate CI, dan PostgreSQL managed/backup cron.
- **Arsipkan `add-storage-service-spec`** → `openspec/changes/archive/`. Semua task sudah `[x]`,
  clippy pass, capability `storage-*` sudah terimplementasi dan di-wire di `rejki-app`.
- **Arsipkan `fix-phase2-code-review-findings`** → `openspec/changes/archive/`. 45/46 task `[x]`;
  satu-satunya task tersisa adalah arsip itu sendiri. Capability `common-errors::TooManyRequests`,
  `auth-token-integrity`, `otp-verification` (attempts tidak di-reset resend) sudah terimplementasi.
- **Finalisasi status sisa task `fix-auth-service-code-review-findings` dan
  `fix-user-service-code-review-findings`**: tandai 4+11 task test/coverage yang deferred-by-design
  dengan anotasi eksplisit bahwa kepemilikan dipindahkan ke `ws-testing-coverage` (P2) — bukan
  ditandai `[x]`. Kedua change ini TIDAK diarsipkan di workstream ini karena masih ada task terbuka
  non-test (verifikasi final) yang akan ditutup bersamaan dengan P2.

## Capabilities

### New Capabilities
<!-- Tidak ada. Workstream ini bersifat housekeeping (dokumentasi + arsip), tidak menambah
     capability fungsional baru. -->

### Modified Capabilities
<!-- Tidak ada modifikasi capability. Status implementasi capability yang sudah ada hanya
     direfleksikan secara akurat di dokumentasi dan checklist. -->

## Impact

- **Dokumentasi**: `docs/index.html` (Master Checklist + footer timestamp) diperbarui untuk
  merefleksikan status sebenarnya. Tidak ada dokumen `docs/*.html` standar lain yang diubah.
- **OpenSpec**: `add-storage-service-spec` dan `fix-phase2-code-review-findings` dipindahkan ke
  `openspec/changes/archive/`. `fix-auth-service-code-review-findings` dan
  `fix-user-service-code-review-findings` tetap aktif dengan anotasi transfer kepemilikan task
  coverage ke P2.
- **Kode**: TIDAK ADA perubahan kode sumber Rust/Bun. Tidak ada migrasi DB. Tidak ada perubahan API.
- **Risiko**: Sangat rendah. Risiko utama hanya ketidakkonsistenan antara checklist dan kode —
  oleh karena itu setiap perubahan label "Selesai" DIDAHULUI verifikasi keberadaan file/symbol
  di kode (bukan asumsi dari dokumen).
- **Standar**: Mematuhi CLAUDE.md §0.1 (jangan asumsi — verifikasi file/symbol) dan §8 alur kerja
  (archive setelah terverifikasi & ter-deploy).
