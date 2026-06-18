# Code Review: `extend-user-suspension-bulk-purge`

**Tanggal**: 2026-06-15
**Reviewer**: Claude Code (cc/claude-opus-4-8)
**Scope**: Seluruh implementasi change `extend-user-suspension-bulk-purge` (auth-service + user-service-client + user-service + rejki-app + test)
**Status**: ✅ PASSED (setelah perbaikan) — 4 temuan ditemukan & diperbaiki dalam review ini

---

## Ringkasan Eksekutif

Change ini mengimplementasi **bulk suspend pengguna** dengan partial-success dan **auto-purge dokumen KYC** saat suspend permanen. Tiga layer utama: (1) refactor suspend single menjadi `suspend_one` reusable, (2) endpoint bulk, (3) `UserClient::purge_kyc_documents` lintas-service via in-process client.

Review **mendalam** (bukan sekadar membaca tasks.md) menemukan **4 temuan nyata** yang luput dari catatan implementasi awal. Semua telah **diperbaiki dan diverifikasi** dalam sesi review ini.

**Hasil Akhir**: 44 test lulus (9 bulk + 13 auth + 9 barang + 13 admin_kyc), clippy clean, fmt clean — terhadap PostgreSQL live (Podman).

---

## Temuan & Perbaikan

### 🔴 T1 — Notifikasi email TIDAK dikirim (pelanggaran spec D5)

**Severity**: High — pelanggaran requirement eksplisit.

**Temuan**: Spec `bulk-user-suspension` ("Notifikasi otomatis ke pengguna ter-suspend") dan design **D5** mewajibkan notifikasi **email + in-app**. Implementasi awal `suspend_accounts_bulk` hanya memanggil `notifier.send(...)` (in-app), **tidak pernah** memanggil `send_email(...)`. Email tidak pernah terkirim.

**Akar masalah**: `suspend_one` mengembalikan `()`, sehingga email pengguna tidak tersedia di loop bulk untuk membangun `EmailMessage { to, .. }`.

**Perbaikan** (`auth-service/src/application/service.rs`):
- `suspend_one` kini mengembalikan `Result<String, _>` (email pengguna).
- `suspend_account` (single) memetakan `.map(|_email| ())` agar kontrak lama tetap.
- Helper privat baru `notify_suspended(notifier, user_id, email, permanent, reason)` mengirim **in-app + email**, keduanya best-effort + logged.
- Teks notifikasi disentralisasi ke `mod suspend_notice` (konstanta `SUBJECT`/`TITLE_*`/`TAIL_*`) → zero hardcoded di alur bisnis (poin 16).

### 🔴 T2 — Jalur purge (D4) tidak ter-cover integration test

**Severity**: High — fitur inti UU PDP tidak teruji; risiko regresi senyap.

**Temuan**: Task 5.4 mengklaim "Test purge terpicu saat permanen", namun:
1. Hanya ada test **negatif** (temp → tidak purge). Tidak ada test **positif** (permanen → dokumen terhapus).
2. **Harness test (`tests/common/mod.rs`) memakai `router_with_deps` tanpa `user_client`** — sehingga `params.user_client = None` dan blok purge **tidak pernah dieksekusi** di test. Test negatif lulus secara *trivial* (purge memang no-op).

**Bukti**: Test positif yang saya tambahkan **gagal** pada kondisi awal (dokumen tidak terhapus), membuktikan purge tidak aktif di harness.

**Perbaikan**:
- `tests/common/mod.rs`: bangun `UserInProcessClient` (mirror `main.rs`) dan inject via `router_with_deps_ex(..., Some(user_client))`. Shared `region_client`/`storage_client` di-reuse.
- `bulk_suspend_test.rs`: tambah `test_bulk_suspend_given_permanent_when_suspend_then_documents_purged` — seed dokumen, suspend permanen, assert `ktp_object_key`/`selfie_object_key` dikosongkan. **Kini lulus.**

### 🟠 T3 — Swagger tidak diperbarui untuk endpoint bulk (poin 26 wajib)

**Severity**: Medium — poin review #26 ("Wajib update Swagger") tidak terpenuhi; task 4.1 sepihak menandai non-goal.

**Temuan**:
1. Endpoint bulk tidak ada di `openapi.rs` sama sekali (padahal padanan iklan — `suspend_pekerjaan_doc` — terdokumentasi; preseden ada).
2. Path doc `suspend_doc`/`suspend_evidence_doc` **salah**: tertulis `/auth/admin/users/{id}/suspend` padahal route aktual saat itu `/auth/users/{id}/suspend`.

**Perbaikan** (`rejki-app/src/openapi.rs`):
- Tambah schema `BulkSuspendDocRequest`, `BulkSuspendItemDocResponse`, `BulkSuspendDocResponse` + path fn `suspend_bulk_doc`, terdaftar di derive `OpenApi` (`paths(...)` + `components(schemas(...))`).
- Path doc disinkronkan ke route final (lihat T4).

### 🟠 T4 — Inkonsistensi path: `admin` hilang dari URL (jawaban atas pertanyaan user)

**Severity**: Medium — kontrak API menyimpang dari konvensi & dari proposal sendiri.

**Temuan**: Endpoint admin-only diletakkan di `/api/v1/auth/users/{id}/suspend` & `/api/v1/auth/users/suspend` — **tanpa** segmen `admin`. Ini menyimpang dari:
- **Konvensi codebase** (konsisten di semua domain lain): `/auth/admin/login`, `/users/admin/kyc/...`, `/admin/pekerjaan/suspend`, `/pelatihan/admin/...`, `/admin/articles`.
- **Proposal & design-nya sendiri** yang menulis `POST /api/v1/auth/admin/users/suspend`.

Hanya endpoint suspend ini yang melanggar pola. URL tidak merefleksikan bahwa akses dibatasi admin.

**Keputusan** (dikonfirmasi user): pindah ke `/auth/admin/users/...`.

**Perbaikan**:
- `auth-service/src/interface/mod.rs`: route admin di-nest dengan prefix `/admin/users/...` → path final `/api/v1/auth/admin/users/suspend`, `/{id}/suspend`, `/{id}/suspend/evidence`.
- `openapi.rs`: path doc disesuaikan.
- 9 URL test di `bulk_suspend_test.rs` disesuaikan. Tidak ada klien produksi yang terdampak (hanya test).

---

## Pemeriksaan Sistematis (35 Poin)

### Arsitektur & Domain (Poin 1-7, 14)

| Poin | Item | Status | Catatan |
|------|------|--------|---------|
| 1 | Proposal terimplementasi | ✅ (setelah perbaikan) | 2 spec delta, 5 requirement, 9 scenario. D5 (email) & path semula menyimpang → diperbaiki (T1, T4). |
| 2 | rejki-app tanpa domain client (composition root) | ✅ PASS | `main.rs` hanya inject `Arc<dyn UserClient>`; tidak akses repo/service lintas domain langsung. |
| 3 | Service sesuai domain | ✅ PASS | auth = akun/suspend; user = profil/KYC/dokumen. Boundary jelas. |
| 4 | Komunikasi via `*-service-client` | ✅ PASS | `UserClient` trait — auth tidak tahu internal user-service. |
| 5 | Clean Architecture | ✅ PASS | Domain trait → Application → Interface → Composition Root. |
| 6 | Rust coding rules | ✅ PASS | `#[async_trait]`, derive standar, tanpa `unsafe`. |
| 7 | SOLID | ✅ PASS | SRP: `notify_suspended` & `suspend_one` terpisah. DIP via trait. |
| 14 | Circular dependency | ✅ PASS | `user-service-client` tidak impor siapa pun. Tanpa cycle. |

### Keamanan & Concurrency (Poin 8-13, 20)

| Poin | Item | Status | Catatan |
|------|------|--------|---------|
| 8 | Race condition | ✅ PASS | Tiap user diproses independen; partial-success. Batch non-transaksional (didokumentasikan di design). |
| 9 | Deprecated/experimental crate | ✅ PASS | Semua crate stabil. |
| 10 | Memory leak | ✅ PASS | `Arc` ref-counting; tanpa `Box::leak`. |
| 12 | Thread safety | ✅ PASS | Semua `Arc<dyn Trait>: Send + Sync`; tanpa `Rc`/`RefCell`. |
| 13 | Connection leak | ✅ PASS | Query via pool; dikembalikan otomatis. |
| 20 | SQL injection | ✅ PASS | Bind parameter (`$1`,`$2`) — purge resolve via repo method, bukan string-concat. |

### Kualitas Kode (Poin 15-19, 21-23)

| Poin | Item | Status | Catatan |
|------|------|--------|---------|
| 15 | Too many arguments | ✅ PASS | `BulkSuspendParams` struct grouping. `notify_suspended` 5 param (< 7). |
| 16 | Zero hardcoded | ✅ (setelah perbaikan) | Teks notifikasi dipindah ke `mod suspend_notice` (T1). `MAX_BULK_SUSPEND_USERS = 100`. *(Catatan minor: `#[validate(length(max = 100))]` masih literal karena attribute butuh const-expr; nilai dijaga sinkron via konstanta dokumentasi.)* |
| 17 | Zero God Function | ✅ PASS | `notify_suspended` diekstrak; `suspend_accounts_bulk` loop tetap ramping. |
| 18 | Zero God Class | ✅ PASS | `AuthService` concern terpisah per method. |
| 19 | Zero Cross-Schema Query | ✅ PASS | auth → `auth.*`; user → `user_svc.*`; lintas via `UserClient`. |
| 21 | Tipe data optimal | ✅ PASS | `Vec<Uuid>`, `Option<&dyn Trait>`, `DateTime<Utc>`. |
| 22 | Algoritma optimal | ✅ PASS | O(n) single pass; purge/notif best-effort tak blocking. |
| 23 | Script query optimal | ✅ PASS | `UPDATE/INSERT` by PK index. |

### Test & Error Handling (Poin 24-25, 30-32)

| Poin | Item | Status | Catatan |
|------|------|--------|---------|
| 24 | Linting | ✅ PASS | `cargo clippy --workspace --all-targets` bersih (live DB). |
| 25 | Formatting | ✅ PASS | `cargo fmt --check` bersih. |
| 30 | Error handling | ✅ PASS | Fail-fast validasi; per-item `success/error`; notif/purge best-effort + `tracing::warn`. |
| 31 | Full coverage | ✅ (setelah perbaikan) | 9 integration test (termasuk **purge positif** baru, T2). |
| 32 | Regression | ✅ PASS | 44 test lulus (9 bulk + 13 auth + 9 barang + 13 admin_kyc) — 0 regresi. |

### Swagger, Dokumentasi, Podman (Poin 26-29, 34-35)

| Poin | Item | Status | Catatan |
|------|------|--------|---------|
| 26 | Swagger | ✅ (setelah perbaikan) | Endpoint bulk + single + evidence terdokumentasi; path dikoreksi (T3, T4). |
| 27 | Web Dashboard state | N/A | Backend-only. |
| 28 | UI Responsive | N/A | Backend-only. |
| 29 | Podman Integration Test | ✅ PASS | PostgreSQL 17 via podman machine; 44 test live. |
| 34 | Update file terkait | ✅ PASS | tasks.md (2.4/2.5/4.1/5.4), openapi.rs, mod.rs, service.rs, test harness, file review ini. |
| 35 | Code review markdown | ✅ PASS | File ini. |

---

## Verifikasi Akhir

```
cargo clippy --workspace --all-targets   → clean (0 error, 0 warning)
cargo fmt --check                        → clean
cargo test -p rejki-app                  → 44 passed; 0 failed
  - bulk_suspend_test    : 9 passed (incl. purge positif)
  - auth_integration_test: 13 passed
  - barang_gratis_test   : 9 passed
  - user_admin_kyc_test  : 13 passed
```

DB: `postgres://rejki@localhost:5432/rejki_db` (Podman, `podman-machine-default` running).

---

## Verdict Final

| Kategori | Skor |
|----------|------|
| Arsitektur (Clean Arch, SOLID, Domain Boundary) | ✅ 10/10 |
| Keamanan (Race, SQLi, Thread Safety) | ✅ 10/10 |
| Performa (Algoritma, Query) | ✅ 10/10 |
| Kualitas Kode (Idioms, Zero Hardcoded, Lint) | ✅ 10/10 |
| Test Coverage (Integration + Regression) | ✅ 10/10 (setelah T2) |
| Kepatuhan Spec & Dokumentasi | ✅ 10/10 (setelah T1/T3/T4) |

**Kesimpulan**: Setelah perbaikan **4 temuan** (T1 email D5, T2 coverage purge, T3 Swagger, T4 path admin), implementasi `extend-user-suspension-bulk-purge` **LULUS** code review. Catatan implementasi awal yang menyatakan "0 temuan" tidak akurat — review mendalam menemukan pelanggaran spec nyata (email tidak terkirim) dan jalur purge yang tidak teruji. Keduanya kini benar dan terverifikasi.
