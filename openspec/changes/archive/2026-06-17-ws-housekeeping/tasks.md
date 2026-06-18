# Tasks: ws-housekeeping

> **Ref:** proposal.md, design.md
> **Aturan verifikasi (CLAUDE.md §0.1):** Setiap penanda "Selesai" pada checklist DIDAHULUI
> verifikasi keberadaan file/symbol di kode. Tidak ada asumsi dari dokumen semata.
> **Tidak ada perubahan kode sumber** pada workstream ini — murni dokumentasi + arsip.

## 1. Verifikasi & perbaiki Master Checklist `docs/index.html`

### 1.1 — Verifikasi item "Structured logging JSON + Request ID per request"
- [x] Verifikasi `request_id_layer` ada di `rust-services/common/tracing-setup/src/lib.rs`
- [x] Verifikasi output JSON pada `production` / pretty pada `development` di `common/tracing-setup`
- [x] Tandai checklist "Structured logging JSON + Request ID per request" → **✓ Selesai**

### 1.2 — Verifikasi item "Multi-stage Dockerfile per service (image <30MB)"
- [x] Verifikasi `rust-services/Dockerfile` multi-stage (dep-cache layer, non-root) ada
- [x] Verifikasi `bun-notification-service/Dockerfile` multi-stage Bun ada
- [x] Tandai checklist → **✓ Selesai** (ukuran image aktual belum terukur di environment ini)

### 1.3 — Verifikasi item "GitHub Actions: fmt → clippy → test → build → push image"
- [x] Verifikasi `.github/workflows/ci.yml` ada (fmt + clippy + test + docker-build-check)
- [x] Verifikasi `.github/workflows/deploy.yml` (SSH deploy prod saat push main) ada
- [x] Verifikasi `.github/workflows/deploy-dev.yml` (SSH deploy dev saat push develop) ada
- [x] Tandai checklist → **✓ Selesai**

### 1.4 — Verifikasi item "nginx.dev.conf + nginx.prod.conf lengkap (WS proxy, rate limit)"
- [x] Verifikasi `nginx.dev.conf` ada (proxy + WS, access_log off untuk WS)
- [x] Verifikasi `nginx.prod.conf` ada (rate-limit 3 zona, JSON log, security headers, WS proxy)
- [x] Tandai checklist → **✓ Selesai**

### 1.5 — Verifikasi item "Cloudflare Tunnel: TUNNEL_TOKEN + zero-trust config"
- [x] Verifikasi referensi cloudflare-tunnel di `docker-compose.yml` (profile:prod) ada
- [x] Verifikasi TUNNEL_TOKEN terdaftar sebagai env/secret (di `docs/vps-deployment.html` dan `docs/infrastructure-setup.html`)
- [x] Tandai checklist → **✓ Selesai**

### 1.6 — Pertahankan "Pending" untuk item yang benar-benar belum
- [x] Konfirmasi "Rate limiting (tower-governor) + OTP brute-force protection" ditandai **⚡ Partial**
      (rate limit hanya ada di auth-service, belum global — akan diselesaikan `ws-security-hardening` P3)
- [x] Konfirmasi "Integration test dengan DB real (schema rejki_test)" ditandai **⚡ Partial**
      (52 test di rejki-app, 0 di crate service — akan diselesaikan `ws-testing-coverage` P2)
- [x] Konfirmasi "PostgreSQL managed DB + backup cron" ditandai **Deferred**

### 1.7 — Perbarui footer timestamp `docs/index.html`
- [x] Ubah footer "Terakhir diperbarui" ke 17 Juni 2026 dan tambahkan catatan: "Phase 3 hardening dimulai — checklist dikoreksi sesuai kondisi kode nyata"

## 2. Arsipkan change yang siap-diarsipkan

### 2.1 — Arsipkan `add-storage-service-spec`
- [x] Verifikasi ulang semua task di `tasks.md` = `[x]` (tidak ada `[ ]`)
- [x] Verifikasi `cargo clippy -p storage-service -p storage-service-client -- -D warnings` pass
- [x] Pindahkan direktori `openspec/changes/add-storage-service-spec/` →
      `openspec/changes/archive/2026-06-17-add-storage-service-spec/`
- [x] Tambahkan catatan status final di arsip (CODE_REVIEW.md tidak dibuat — change murni spec verification)

### 2.2 — Arsipkan `fix-phase2-code-review-findings`
- [x] Verifikasi 45/46 task `[x]`; satu-satunya `[ ]` adalah task arsip itu sendiri
- [x] Verifikasi capability `common-errors::TooManyRequests` (429), `auth-token-integrity`
      (refresh expiry), `otp-verification` (attempts tidak di-reset resend) sudah terimplementasi
      di kode (spot-check di `auth-service`)
- [x] Pindahkan direktori `openspec/changes/fix-phase2-code-review-findings/` →
      `openspec/changes/archive/2026-06-17-fix-phase2-code-review-findings/`

## 3. Finalisasi status sisa task `fix-auth-service-code-review-findings`

### 3.1 — Anotasi transfer kepemilikan task coverage ke P2
- [x] Tandai task 2.4 (concurrent refresh integration test) dengan anotasi: "deferred →
      `ws-testing-coverage` (P2) section 2 — integration tests live in rejki-app"
- [x] Tandai task 8.16 (coverage measurement) dengan anotasi: "deferred → `ws-testing-coverage`
      (P2) section 3 — coverage gate CI"
- [x] Tandai task 8.17 (integration tests) dengan anotasi: "deferred → `ws-testing-coverage` (P2)
      section 2"
- [x] Tandai task 11.5 (coverage report llvm-cov) dengan anotasi: "deferred → `ws-testing-coverage`
      (P2) section 3 — tooling nightly/tarpaulin"
- [x] **DONE:** 4 task fix-auth dianotasi transfer ke P2. TETAP `[ ]` — P2 mengambil alih.

## 4. Finalisasi status sisa task `fix-user-service-code-review-findings`

### 4.1 — Anotasi transfer kepemilikan task test/coverage ke P2
- [x] Tandai task (integration test NIK guard, baris 14) dengan anotasi "deferred → P2 section 2"
- [x] Tandai task (integration test partial failure NIK, baris 25) dengan anotasi "deferred → P2 section 2"
- [x] Tandai task `#[cfg(test)] mod tests` infrastructure (baris 80) → "deferred → P2 section 1"
- [x] Tandai task `#[cfg(test)] mod tests` interface (baris 83) → "deferred → P2 section 1"
- [x] Tandai task coverage ≥85% (baris 84) → "deferred → P2 section 3"
- [x] Tandai task CI coverage gate (baris 91) → "deferred → P2 section 3"
- [x] Tandai task unit test CSV escape (baris 91) → "deferred → P2 section 1"
- [x] Tandai task `cargo test -p rejki-app --test auth_integration_test` (baris 120) → "deferred → P2 section 2"
- [x] Tandai task `cargo test -p rejki-app --test bulk_suspend_test` (baris 121) → "deferred → P2 section 2"
- [x] Tandai task archive (baris 130) → "tetap `[ ]` hingga P2 selesai, baru archive"
- [x] Tandai task "Notify team consumer publik endpoint" (baris 35) → tetap `[ ]`, tindakan komunikasi tim (bukan code) — selesaikan terpisah

## 5. Verifikasi & laporan akhir

- [x] Konfirmasi `openspec/changes/` (non-archive) berisi: `ws-housekeeping`, `ws-testing-coverage`, `fix-auth-service-code-review-findings`, `fix-user-service-code-review-findings`, `archive/`
- [x] Jalankan `cargo check --workspace` — ✅ PASS, semua service compile clean
- [x] Buat ringkasan singkat: 5 item checklist berubah status, 2 change terarsip, 15 task di-anotasi transfer ke P2

**✅ ws-housekeeping COMPLETE.** Semua task (kecuali item "Notify team" & archive fix-auth/fix-user yang menunggu P2) selesai. P1 siap ditutup, P2 (`ws-testing-coverage`) dapat dimulai.
