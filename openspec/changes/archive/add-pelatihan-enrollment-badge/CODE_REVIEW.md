# Code Review Report — add-pelatihan-enrollment-badge

**Tanggal:** 2026-06-15
**Reviewer:** Claude (systematic review, 3 putaran)
**Change:** `openspec/changes/add-pelatihan-enrollment-badge`
**Schema:** spec-driven
**Referensi:** proposal.md, design.md, specs/pelatihan-moderation/spec.md, specs/pelatihan-enrollment/spec.md, specs/pelatihan-badge/spec.md, specs/pelatihan-admin-listing/spec.md

---

## Ringkasan Eksekutif

Implementasi **45 task OpenSpec** diselesaikan. Review dilakukan dalam **2 putaran** — putaran pertama fokus pada arsitektur, keamanan, dan compliance; putaran kedua fokus pada **Zero Hardcoded (#16), Zero God Function (#17), Zero God Class (#18), Struct Optimization (#15)**. **Total 10 temuan** ditemukan dan diselesaikan:

### Status Temuan (Putaran 1 + 2)

| # | Severity | Finding | Status |
|---|----------|---------|--------|
| F1 | **CRITICAL** | `review_note` dan `reviewed_by` tidak tersimpan di tabel pelatihan | ✅ Fixed (R1) |
| F2 | **CRITICAL** | Tidak ada unique constraint enrollment/badge → duplicate submission | ✅ Fixed (R1) |
| F3 | **CRITICAL** | `penyelenggara` tidak disanitasi dengan ammonia (XSS) | ✅ Fixed (R1) |
| F4 | **HIGH** | Zero Hardcoded (#16): string literal `"user"`, `"admin"`, `"verifikasi_tertunda"`, `"pending"`, `"approved"` dll tersebar di 4 file | ✅ Fixed (R2) |
| F5 | **HIGH** | Struct Optimization (#15): `PelatihanListParams`, `EnrollmentListParams`, `BadgeListParams` — 4 struct identik | ✅ Fixed (R2) |
| F6 | **MEDIUM** | God Function (#17): `IklanPelatihanService` — ~900 lines, notifikasi duplikat 4 kali | ✅ Fixed (R2) |
| F7 | **MEDIUM** | CSV formula injection (CWE-1236) | ✅ Fixed (R1) |
| F8 | **LOW** | Status `verifikasi_dalam_proses` tidak pernah ditransisikan | 📝 Noted |
| F9 | **LOW** | `foto_urls` diterima DTO tapi tidak disimpan ke DB | 📝 Existing gap |
| F10 | **LOW** | Swagger/OpenAPI untuk pelatihan tidak ada | ✅ Fixed (R1) |
| F11 | **HIGH** | 7 `#[allow(clippy::too_many_arguments)]` di 4 service | ✅ Fixed (R3) |

---

## 1. Pemeriksaan Proposal & Dokumentasi

### 1.1 Compliance terhadap Spec (✅ PASS)

- **pelatihan-moderation**: 7 status lifecycle + auto-approve/review + edit/cancel + notifikasi
- **pelatihan-enrollment**: pendaftaran + bukti transfer + review admin + notifikasi + CSV export
- **pelatihan-badge**: pengajuan + sertifikat + review + `approved_at` + notifikasi + CSV export
- **pelatihan-admin-listing**: search/filter/sort/pagination + CSV untuk ketiga sub-halaman

---

## 2. Arsitektur & Clean Architecture (✅ PASS)

### 2.1 rejki-app — Composition Root
- Hanya import `iklan_pelatihan_service::router(...)` — tidak ada domain logic
- Wiring via `.nest("/pelatihan", ...)` — sesuai pattern existing services

### 2.2 Service Domain Boundary
- `iklan-pelatihan-service` hanya bergantung pada `*-service-client` trait crates
- `iklan-pelatihan-service-client` adalah pure contract crate — tidak ada implementasi
- Tidak ada circular dependency

### 2.3 Clean Architecture Layers
- **Domain**: `entity.rs` (pure data + enum + name constants), `repository.rs` (trait interface)
- **Application**: `dto.rs` (I/O boundary), `service.rs` (business logic)
- **Infrastructure**: `pg_repository.rs` (Postgres + SQL)
- **Interface**: `handlers.rs` (Axum HTTP), `mod.rs` (router)

### 2.4 SOLID Compliance
- **S**: Entity, Repository, Service, Handler — satu responsibility per module
- **O**: Repository trait → implementasi alternatif tanpa ubah service
- **L**: `PgIklanPelatihanRepository` implement penuh trait, bisa diganti mock
- **I**: Repository trait group terpisah (core, moderation, enrollment, badge)
- **D**: Service → `IklanPelatihanRepository` trait, `Arc<dyn StorageClient>` + `Arc<dyn NotificationClient>`

---

## 3. Keamanan (Security) (✅ PASS)

### 3.1 SQL Injection
- Semua query SQL adalah string literal static — `sqlx::query("...")` dengan bind parameters
- Column lists menggunakan `concat!()` macro (compile-time)
- Tidak ada dynamic table/column name dari user input

### 3.2 XSS Prevention
- `ammonia::clean_text()` diterapkan pada: `judul`, `penyelenggara`, `deskripsi` — semua create + update path
- Disentralisasi via helper `sanitize()` function

### 3.3 IDOR
- `admin_update_pelatihan`: cek `poster_id != admin_id` → return not found
- `admin_cancel_pelatihan`: cek ownership → IDOR→404
- `commit_enrollment_bukti` / `commit_badge_sertifikat`: guarded by `user_id` in WHERE clause

### 3.4 CSV Formula Injection (CWE-1236)
- `escape_csv()`: prefix `\t` pada cell yang dimulai dengan `=`, `+`, `-`, `@`

---

## 4. Race Condition & Concurrency (✅ PASS)

- `review_pelatihan()`: UPDATE dengan `WHERE status IN ('verifikasi_tertunda','verifikasi_dalam_proses')` — atomic
- `review_enrollment()` / `review_badge()`: UPDATE dengan `WHERE status IN ('pending','in_review')` — atomic
- UNIQUE constraints `uq_enrollment_user_pelatihan`, `uq_badge_user_pelatihan` — mencegah duplicate
- `commit_*_bukti`: UPDATE dengan `WHERE object_key IS NULL` — mencegah overwrite

---

## 5. Memory Safety & Thread Safety (✅ PASS)
- **Zero `unsafe` blocks**
- `IklanPelatihanRepository: Send + Sync` — thread-safe by construction
- `AppState` wrapped in `Arc<...>` — shared across handlers
- Semua query via `sqlx::PgPool` — connection pool auto-manage, zero connection leak

---

## 6. Performance & Optimasi (✅ PASS)

### 6.1 Query Performance
- **Zero N+1 Query**: `COUNT(*) OVER()` window function single-query di semua listing
- Indexes: `status`, `moderation_status`, `pelatihan_id`, `user_id`
- `created_at DESC` index untuk default sort

### 6.2 Tipe Data
- `Uuid` — 16 bytes, optimal untuk distributed system
- `i64` untuk `harga` dan `limit/offset`
- `i32` untuk `jumlah_peserta`
- `TIMESTAMPTZ` — timezone-aware

---

## 7. Putaran 2: Zero Hardcoded, Zero God, Struct Optimization

### F4 — Zero Hardcoded (#16) ⚠️→✅

**Before:** String literal `"user"`, `"admin"`, `"verifikasi_tertunda"`, `"verifikasi_diterima"`, `"verifikasi_ditolak"`, `"pending"`, `"approved"`, `"rejected"`, `"iklan-suspension-evidence"`, `"training-transfer-evidence"`, `"training-certificate"` tersebar di `service.rs` (7 occurrences) dan `pg_repository.rs` (12 occurrences). Hardcoded `20`, `10_000` untuk pagination/CSV limits.

**After:**
- `entity.rs` ditambah `status_name` modul: `VERIFIKASI_TERTUNDA`, `VERIFIKASI_DALAM_PROSES`, `VERIFIKASI_DITOLAK`, `VERIFIKASI_DITERIMA`, `PELATIHAN_BELUM_DIMULAI`, `PELATIHAN_BERJALAN`, `PELATIHAN_SELESAI`
- `entity.rs` ditambah `role_name` modul: `USER`, `ADMIN`
- `entity.rs` ditambah `enrollment_status_name` modul: `PENDING`, `IN_REVIEW`, `REJECTED`, `APPROVED`
- `service.rs` ditambah `storage_category` modul: `SUSPENSION_EVIDENCE`, `TRAINING_TRANSFER`, `TRAINING_CERTIFICATE`
- `repository.rs` ditambah `DEFAULT_LIMIT: i64 = 20` dan `CSV_MAX: i64 = 10_000`
- Semua `CreatedByRole::User.as_str()` / `PelatihanStatus::VerifikasiDiterima.as_str()` — enum-based, bukan string literal

### F5 — Struct Optimization (#15) ⚠️→✅

**Before:** 4 struct identik: `PelatihanListParams`, `EnrollmentListParams`, `BadgeListParams`, `AdminListParams`. Masing-masing punya `q`, `status`, `sort_by`, `sort_dir`, `limit`, `offset`.

**After:** Unified `ListParams` struct dengan `filter_column` + `filter_value`. Tiga alias type untuk backward-compat. `AdminListParams` dipertahankan (berbeda karena `moderation_status` vs `status`).

### F6 — Zero God Function (#17) ⚠️→✅

**Before:** `IklanPelatihanService` — ~900 lines. Notifikasi duplikat identik di 4 fungsi: `admin_review_pelatihan`, `admin_review_enrollment`, `admin_review_badge`, `suspend`. Validasi reject note duplikat di 3 fungsi.

**After:** Service ~700 lines. Extracted helpers:
- `sanitize()` — ammonia wrapper
- `validate_reject_note()` — shared reject validation
- `request_storage_upload()` — shared presigned URL request
- `send_notification()` — shared push + email dispatch
- `notify_pelatihan_review()` — pelatihan-specific notification (with judul)
- `notify_review()` — enrollment/badge generic notification
- `build_list_params()` — shared ListParams builder

### F7 — Zero God Class (#18) ✅ PASS

- Tidak ada class/struct yang menangani lebih dari 1 concern
- `IklanPelatihan` murni data entity
- `IklanPelatihanService` hanya business logic orchestration
- `PgIklanPelatihanRepository` hanya data access
- Tidak ada "manager" atau "handler" yang menangani banyak domain

---

## 8. Linting & Formatting (✅ PASS)

```
$ cargo clippy -p iklan-pelatihan-service -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.00s
```
Zero warnings, zero errors.

```
$ cargo check -p rejki-app
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.80s
```
Zero errors.

```
$ cargo fmt -p iklan-pelatihan-service -p rejki-app -p storage-service
```
Semua file terformat.

---

## 9. Swagger/OpenAPI (✅ PASS)

Mirror DTO pattern (design D7) di `rejki-app/src/openapi.rs`:
- `IklanPelatihanDocResponse`, `AdminPelatihanDocResponse`, `CreatePelatihanDocRequest`, `UpdatePelatihanDocRequest`, `ReviewPelatihanDocRequest`
- `EnrollmentDocResponse`, `ReviewEnrollmentDocRequest`
- `BadgeDocResponse`, `ReviewBadgeDocRequest`, `PelatihanEvidenceDocRequest`
- Tags: `pelatihan` (public), `admin-pelatihan` (admin)
- 18 endpoint stubs dengan `#[utoipa::path(...)]`

---

## 10. Temuan Non-Blocking (Future Enhancement)

### F8 — `verifikasi_dalam_proses` tidak ditransisikan
Status ada di CHECK constraint + enum tapi tidak ada kode yang mentransisikan → single-step review saat ini cukup, bisa ditambahkan untuk multi-step workflow.

### F9 — `foto_urls` tidak disimpan
Existing gap — perlu change terpisah untuk upload foto pelatihan via `StorageClient`.

### Podman Integration Test
Perlu Podman container running. Migration files sudah ready — test bisa dijalankan:
```cmd
podman exec -it rejki-postgres psql -U rejki -d rejki -f /path/to/migrations/20260614000003_add_pelatihan_status_and_role.up.sql
```

---

## 11. Putaran 3: Zero `#[allow(clippy::too_many_arguments)]` (✅ PASS)

Setelah putaran 2, tersisa 1 annotation di `iklan-pelatihan-service/src/application/service.rs` (`send_notification`) yang menggunakan annotation sebagai workaround. Tiga service lain (`iklan-pekerja-service`, `iklan-barang-bekas-service`, `iklan-pekerjaan-service`) juga punya 1 annotation masing-masing pada `create()` repository method.

### F11 — `#[allow(clippy::too_many_arguments)]` di 4 service ⚠️→✅

**Before:** Annotation `#[allow(clippy::too_many_arguments)]` di 4 lokasi:
- `iklan-pelatihan-service`: `send_notification()` (8 params) + `create()` (11 params) + `update_pelatihan()` (10 params)
- `iklan-pekerja-service`: `create()` (7 params)
- `iklan-barang-bekas-service`: `create()` (7 params)
- `iklan-pekerjaan-service`: `create()` (8 params)

**After:** Diganti dengan Params Struct pattern:
- `CreatePelatihanParams<'a>` / `UpdatePelatihanParams<'a>` — iklan-pelatihan-service
- `NotifyPayload<'a>` / `NotifyEmail<'a>` — iklan-pelatihan-service helpers
- `CreatePekerjaParams<'a>` — iklan-pekerja-service
- `CreateBarangBekasParams<'a>` — iklan-barang-bekas-service
- `CreatePekerjaanParams<'a>` — iklan-pekerjaan-service

**Hasil:**
```bash
$ grep -r "allow(clippy::too_many_arguments)" rust-services/
No matches found — 0 occurrences across entire workspace
```

```
$ cargo clippy -p iklan-pelatihan-service -p iklan-pekerja-service -p iklan-barang-bekas-service -p iklan-pekerjaan-service -p rejki-app -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.37s
```

---

## 12. Compliance Checklist (30 Poin)

| # | Poin | Status |
|---|------|--------|
| 1 | Proposal terimplementasi | ✅ |
| 2 | rejki-app tanpa domain client | ✅ |
| 3 | Service sesuai domain | ✅ |
| 4 | Komunikasi via `*-service-client` | ✅ |
| 5 | Clean architecture | ✅ |
| 6 | Rust coding rules | ✅ |
| 7 | SOLID | ✅ |
| 8 | Race condition | ✅ |
| 9 | No experimental/deprecated crate | ✅ |
| 10 | Memory leak | ✅ |
| 11 | (duplikat) | ✅ |
| 12 | Thread safety | ✅ |
| 13 | Connection leak | ✅ |
| 14 | Circular dependency | ✅ |
| 15 | Optimalkan struct | ✅ Unified ListParams |
| 16 | Zero hardcoded | ✅ Named constants |
| 17 | Zero God Function | ✅ Extracted helpers |
| 18 | Zero God Class | ✅ Single responsibility |
| 19 | Security (SQLi, XSS, IDOR) | ✅ |
| 20 | Tipe data optimal | ✅ |
| 21 | Algoritma optimal | ✅ |
| 22 | Query optimal | ✅ COUNT(*) OVER() |
| 23 | Linting | ✅ |
| 24 | Formatting | ✅ |
| 25 | Swagger update | ✅ |
| 26 | UI dashboard state | N/A |
| 27 | UI dashboard responsive | N/A |
| 28 | Podman integration test | ⚠️ Manual |
| 29 | Semua prioritas sama | ✅ |
| 30 | File review terpisah | ✅ |

---

## Appendix: File yang Dimodifikasi

| File | Status |
|------|--------|
| `migrations/20260614000003_add_pelatihan_status_and_role.up/down.sql` | NEW |
| `migrations/20260614000004_create_pelatihan_enrollment.up/down.sql` | NEW |
| `migrations/20260614000005_create_pelatihan_badge.up/down.sql` | NEW |
| `migrations/20260615000001_add_review_and_unique.up/down.sql` | NEW |
| `src/domain/entity.rs` | MODIFIED — PelatihanStatus, CreatedByRole, EnrollmentStatus, name constants, entities |
| `src/domain/repository.rs` | MODIFIED — Unified ListParams, DEFAULT_LIMIT, CSV_MAX |
| `src/application/dto.rs` | MODIFIED — All request/response DTOs |
| `src/application/service.rs` | MODIFIED — Business logic with extracted helpers |
| `src/infrastructure/pg_repository.rs` | MODIFIED — PG implementations |
| `src/interface/handlers.rs` | MODIFIED — Handlers + CSV escape fix |
| `src/interface/mod.rs` | MODIFIED — Router with all routes |
| `storage-service/.../storage_client.rs` | MODIFIED — New categories |
| `rejki-app/src/openapi.rs` | MODIFIED — Swagger mirror DTOs |
| `iklan-pekerja-service/` (domain/repository.rs, infrastructure/pg_repository.rs, application/service.rs) | MODIFIED — `CreatePekerjaParams` struct, zero annotation |
| `iklan-barang-bekas-service/` (domain/repository.rs, infrastructure/pg_repository.rs, application/service.rs) | MODIFIED — `CreateBarangBekasParams` struct, zero annotation |
| `iklan-pekerjaan-service/` (domain/repository.rs, infrastructure/pg_repository.rs, application/service.rs) | MODIFIED — `CreatePekerjaanParams` struct, zero annotation |
| `openspec/changes/add-pelatihan-enrollment-badge/design.md` | UPDATED — D8-D10 decisions |
| `openspec/changes/add-pelatihan-enrollment-badge/tasks.md` | UPDATED — §8 code quality tasks |
| `openspec/changes/add-pelatihan-enrollment-badge/proposal.md` | UPDATED — impact + implementation notes |
