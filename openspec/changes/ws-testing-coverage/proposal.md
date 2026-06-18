# Proposal: ws-testing-coverage

> **Phase 3 — Workstream 2 (Testing & Coverage)**
> **Depends on:** `ws-housekeeping` (P1) — butuh anotasi transfer kepemilikan task coverage dari
> `fix-auth` & `fix-user` agar tidak duplikasi.
> **Blocks:** `ws-security-hardening` (P3) dan seluruh workstream hilir — setiap perubahan
> fungsional setelahnya WAJIB lulus coverage gate yang dibangun di sini.

## Why

Acceptance Criteria CLAUDE.md §2 mewajibkan **overall coverage ≥ 85%**, dengan breakdown layer
(application ≥ 80%, interface ≥ 70%, infrastructure ≥ 60%). Code review Phase 2 (`fix-auth`,
`fix-user`) menemukan **0% unit test coverage** di crate `auth-service` & `user-service` —
seluruh test ada di `rejki-app/tests/` (integration). Akibatnya:

1. **Pelanggaran Acceptance Criteria yang outstanding.** Tidak ada coverage gate di CI yang
   mencegah regresi. Setiap perubahan Phase 3 berikutnya menambah utang testing tanpa safety net.
2. **CI integration-test hanya menjalankan 1 dari 5 file test.** `ci.yml` job `integration-test`
   hanya run `cargo test --test auth_integration_test`; file `barang_gratis_test`,
   `bulk_suspend_test`, `user_admin_kyc_test`, `user_code_review_fix_test` **belum pernah
   dijalankan di CI** (task `fix-user` 5.2 mencatatnya sebagai "belum dijalankan").
3. **CI schemas tidak sinkron dengan jumlah service.** Step "Init schemas" di `ci.yml` hanya
   membuat 8 schema (auth, user_svc, chat, notification, 4 iklan), padahal workspace sekarang
   punya 12 service (kurang region, corporate-comms, report, storage) → integration test yang
   menyentuh service tersebut akan gagal migrasi.
4. **Task coverage yang didefer** dari `fix-auth` (4 task) & `fix-user` (11 task) menunggu
   workstream ini mengambil alih — anotasi transfer kepemilikan sudah disiapkan di P1.

Tujuan workstream ini: membangun **foundation testing yang lengkap & ter-otomasi** (unit test
per-service crate + integration test per-service + coverage gate CI), sehingga seluruh Phase 3
hilir punya safety net dan Acceptance Criteria ≥ 85% dapat diukur & ditegakkan.

## What Changes

- **Unit test per-service crate** — bangun `#[cfg(test)] mod tests` di layer `application` &
  `domain` tiap `*-service` crate (mulai auth & user — paling banyak business logic), dengan
  `Mock*Repository` (di fixture `tests/`) agar application layer teruji tanpa DB. Naming sesuai
  testing-standard: `test_{unit}_given_{kondisi}_when_{aksi}_then_{ekspektasi}`.
- **Integration test per-service** — pindahkan/jadikan integration test per-service di
  `tests/` pakai DB nyata (`#[sqlx::test]`, schema `rejki_test`), idempoten, fixture factory
  (email `@test.rejki.internal`). Tutup task deferred dari `fix-auth`/`fix-user` (concurrent
  refresh, partial-failure NIK, auth_integration_test, bulk_suspend_test).
- **Sinkronisasi CI schemas** — perbarui step "Init schemas" `ci.yml` ke 12 schema (tambah
  region, corporate-comms, report, storage) + jalankan migrasi semua service yang punya folder
  `migrations/`.
- **CI integration-test menjalankan SEMUA test file** — bukan hanya `auth_integration_test`;
  pakai `cargo test --workspace --test '*'` (atau daftar eksplisit) dengan `--test-threads=1`.
- **Coverage gate CI** — tambah job `coverage` di `ci.yml`: `cargo llvm-cov --workspace --lcov`
  (atau `tarpaulin`), publish report, **fail PR bila overall < 85%**. Threshold per-layer
  (application ≥ 80%, interface ≥ 70%, infrastructure ≥ 60%) dilaporkan sebagai info, gate
  utama = overall ≥ 85% (sesuai Acceptance Criteria §2). Tooling: prefer `cargo-llvm-cov`
  (stable, lebih cepat dari tarpaulin) — tidak butuh nightly (catatan: task `fix-auth` 11.5
  menyebut nightly karena versi lama; `cargo-llvm-cov` modern berjalan di stable).
- **Test IDOR + 429 + rate-limit behavior** per service (wajib per testing-standard & security-
  baseline) — termasuk test yang menutup capability dari `fix-phase2` (`TooManyRequests` 429,
  OTP brute-force via resend, refresh-token expiry enforcement).

## Capabilities

### New Capabilities
- `testing-unit-coverage`: unit test di layer application/domain tiap `*-service` crate dengan
  mock repository, target coverage per-layer (application ≥ 80%, domain ~100%).
- `testing-integration-coverage`: integration test per-service dengan DB nyata (`rejki_test`),
  idempoten, fixture factory; menjalankan seluruh file test di CI.
- `testing-coverage-gate`: CI coverage gate overall ≥ 85% dengan `cargo-llvm-cov`, fail PR bila
  di bawah threshold.

### Modified Capabilities
- `common-ci-pipeline` (`.github/workflows/ci.yml`): sinkronisasi 12 schema, jalankan semua
  integration test file, tambah job coverage.

## Impact

- **Kode**: banyak `#[cfg(test)]` block baru di `auth-service`, `user-service` (prioritas), lalu
  bertahap ke service lain. Tidak ada perubahan API/kontrak. Tidak ada migrasi DB struktural
  (hanya test schema).
- **CI**: `.github/workflows/ci.yml` — perluas schema init, perluas cakupan integration-test,
  tambah job `coverage`. Durasi CI akan naik (coverage instrumentation) — mitigasi: cache
  `rust-cache` + jalankan coverage paralel dengan build check bila memungkinkan.
- **Tooling**: tambah `cargo-llvm-cov` ke CI (install via `cargo install cargo-llvm-cov` + system
  dep `llvm-tools-preview`). Tidak menambah dependency runtime production.
- **OpenSpec**: menutup task deferred dari `fix-auth-service-code-review-findings` (4 task) &
  `fix-user-service-code-review-findings` (11 task) → setelah P2 selesai, kedua change tersebut
  dapat di-archive (tindakan di P2 final, atau P1 lanjutan).
- **Risiko**: sedang. Risiko utama = flaky integration test (DB timing) → mitigasi:
  `--test-threads=1` + idempoten fixture + transaction rollback per test. Risiko kedua = durasi
  CI naik → mitigasi caching.
- **Standar**: mematuhi CLAUDE.md §4.8 (testing tier, naming, fixture, coverage target), §2
  (Acceptance Criteria ≥ 85%), docs/testing-standard.html.
