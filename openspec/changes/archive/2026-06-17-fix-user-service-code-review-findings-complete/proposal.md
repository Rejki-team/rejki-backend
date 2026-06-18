## Why

Code review menyeluruh terhadap `user-service` (Juni 2026) menemukan **10 temuan** — mencakup
**2 race condition** (data integrity), **1 IDOR/information disclosure** (security),
**1 NIK validation bypass** (data quality), **1 `async-trait` misuse** (CLAUDE.md §4.1 violation),
**1 audit trail silent discard** (compliance Q2), **1 business rule mismatch** (calendar days vs business days),
**1 CSV formula injection** (CWE-1236), **0% unit test coverage** (Acceptance Criteria 85% tidak terpenuhi),
dan **1 observability gap** (`warn_slow!` missing).

Temuan ini harus diperbaiki sebelum production deployment karena berpotensi menyebabkan:
- Data inconsistency (NIK overwrite, partial KYC submission tanpa recovery path)
- Information disclosure (public endpoint mengembalikan `phone`, `full_name`, `bio`, `kyc_status`)
- Compliance gap (audit trail tidak utuh — melanggar UU PDP)
- Target coverage 85% tidak tercapai (tidak ada unit test sama sekali di user-service crate)

Target: semua temuan **CRITICAL** selesai dalam sprint ini, **HIGH** dalam sprint berikutnya,
**MEDIUM + LOW** dalam 2 sprint.

## What Changes

**P0 — CRITICAL (data integrity & security):**
- Fix `update_profile` NIK immutability: tambah guard `AND nik_encrypted IS NULL` di level database + return `rows_affected()` untuk signal overwrite (C1)
- Fix `submit_kyc` partial failure: bungkus `find_by_auth_id` + `update_profile` + `create_submission` dalam database transaction (C2)
- Fix public endpoint IDOR: hapus `GET /api/v1/users/{id}` dari public router; pindahkan ke protected router dengan ownership check (C3)

**P1 — HIGH (validation, compliance, standards):**
- Tambah validasi digit-only untuk field NIK (regex atau manual check) — cegah non-digit string diterima sebagai NIK (H1)
- Hapus `async-trait` dari `UserRepository` trait — tidak diperlukan karena trait TIDAK dipakai sebagai `dyn UserRepository`; gunakan native `async fn in trait` (Rust 1.75+) (H2)
- Propagasi error `log_upload_issued` — ganti `let _ =` dengan `?` atau minimal `tracing::warn!` agar audit trail utuh (H3)

**P2 — MEDIUM (business logic, testing, security):**
- Ganti cooldown 3 hari kalender → 3 hari kerja, atau rename konstanta menjadi `KYC_COOLDOWN_CALENDAR_DAYS` jika memang disengaja 3 hari kalender (M1)
- Tambah unit test di user-service crate: `#[cfg(test)] mod tests` di tiap module (domain, application, infrastructure, interface). Target coverage ≥ 85% overall (M2)
- Tambah CSV formula injection protection (CWE-1236): prefix cell formula-prefixed dengan tab (M3)

**P3 — LOW (observability):**
- Tambah `warn_slow!` ke `update_profile` dan `update_avatar` di `PgUserRepository` (L1)

## Capabilities

### New Capabilities
- `user-service-core-integrity`: Database transaction wrapping + atomic NIK guard untuk submit KYC
- `user-service-security`: Anti-IDOR, CSV injection protection, NIK validation hardening
- `user-service-coverage`: Unit test framework untuk user-service crate, target coverage ≥ 85%

### Modified Capabilities
Tidak ada perubahan kontrak API. Perbaikan bersifat internal service:
- `user-profile` (add-user-service-kyc): Public GET /{id} dipindahkan ke protected
- `kyc-personal-data` (add-user-service-kyc): NIK validation diperkuat + transaction wrapping
- `kyc-verification-workflow` (add-user-service-kyc): Cooldown diselaraskan dengan K16
- `kyc-documents` (add-user-service-kyc): Audit trail upload_issued dipropagasi

## Impact

- **Affected code**: `user-service/src/application/service.rs`, `user-service/src/infrastructure/pg_repository.rs`, `user-service/src/interface/handlers.rs`, `user-service/src/interface/mod.rs`, `user-service/src/application/dto.rs`, `user-service/src/domain/repository.rs`, `user-service/Cargo.toml`
- **Affected crates**: `user-service` only (perubahan internal, tidak ada perubahan client crate)
- **Breaking changes**: **Ada minor** — `GET /api/v1/users/{id}` yang sebelumnya publik kini butuh autentikasi. Jika ada consumer internal yang bergantung pada endpoint publik ini, perlu diinformasikan.
- **New dependencies**: Tidak ada (bahkan menghapus `async-trait` dependency jika memungkinkan)
- **Tests affected**: Perlu ditambah unit test baru (M2); existing integration test di `rejki-app/tests/user_admin_kyc_test.rs` harus tetap pass

## References

- CLAUDE.md §2 (Acceptance Criteria), §3 (Boundary Arsitektur), §4 (Rust+Axum), §4.1 (async-trait), §4.4 (IDOR), §4.5 (Transaction/Race Condition), §4.6 (warn_slow!), §4.8 (Unit Test)
- Code Review: `docs/code-review/code-review-user-service-jun-2026.md`
- OpenSpec changes: `add-user-service-kyc`, `add-user-admin-management`
