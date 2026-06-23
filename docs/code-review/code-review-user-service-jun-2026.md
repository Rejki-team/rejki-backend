# Code Review: User Service — Audit Menyeluruh (Juni 2026)

**Tanggal Review:** 2026-06-16
**Reviewer:** Claude Code (DeepSeek V4 Pro Max)
**Branch:** `feature/repo-governance`
**Scope:** `rejki-backend/rust-services/user-service/` — seluruh 14 file + migration + test
**Acuan:** CLAUDE.md §2 (Acceptance Criteria), §3 (Boundary Arsitektur), §4 (Rust+Axum), OpenSpec `add-user-service-kyc` & `add-user-admin-management`, Phase 1 & 1.5

---

## Ringkasan Eksekutif

Review dilakukan terhadap **14 source file** + **7 migration file** + **2 test file** user-service.
Ditemukan **38 kandidat temuan** dari 8 angle finder, setelah dedup → **18 unik**, setelah verifikasi
silang dengan CLAUDE.md → **10 final** (3 CRITICAL, 3 HIGH, 3 MEDIUM, 1 LOW).

**Status Build (sebelum fix):**

| Check | Status |
|-------|--------|
| `cargo check --workspace` | ✅ PASS |
| `cargo clippy --workspace -- -D warnings` | ⚠️ Perlu verifikasi ulang |
| `cargo fmt --check` | ⚠️ Perlu verifikasi ulang |
| `cargo test` (unit) | ⚠️ Tidak ada unit test di user-service crate |
| Integration test (rejki-app) | 13/13 PASS |

---

## Temuan CRITICAL (Harus Diperbaiki)

### C1. 🔴 Race Condition TOCTOU pada `submit_kyc` — `update_profile` tanpa guard atomik

**File:** [pg_repository.rs:261-286](rust-services/user-service/src/infrastructure/pg_repository.rs#L261)
**Aturan CLAUDE.md dilanggar:** §4.5 — "Zero Race Condition: operasi gabungan harus atomik — bungkus dalam transaction"

**Masalah:**
Aplikasi sudah memproteksi TOCTOU lewat single-fetch `find_by_auth_id` + cek `nik_encrypted.is_some()`
(C1 dari review sebelumnya). Namun di level database, `update_profile` menulis NIK via:

```sql
UPDATE user_svc.profiles SET ... nik_encrypted = $3 ... WHERE id = $1
```

Tanpa klausa `AND nik_encrypted IS NULL`. Dua request `submit_kyc` konkuren untuk pengguna yang
sama, keduanya lolos cek aplikasi (`nik_encrypted.is_some() = false`), keduanya memanggil
`update_profile`. Yang kedua **overwrite** NIK yang pertama tanpa error. Ini melanggar K5 (NIK immutable).

**Failure Scenario:**
1. User X belum isi NIK. Dua tab browser mengirim `submit_kyc` bersamaan.
2. Request A: `find_by_auth_id` → nik_encrypted=None ✅, lanjut encrypt NIK_A, `update_profile` writes NIK_A.
3. Request B: `find_by_auth_id` (sebelum commit A) → nik_encrypted=None ✅, lanjut encrypt NIK_B, `update_profile` writes NIK_B (overwrite NIK_A).
4. Profil sekarang berisi NIK_B (yang mungkin berbeda digit). Audit trail tidak merekam overwrite.

**Perbaikan:**
```sql
-- Repository: update_profile harus guarded di db level
UPDATE user_svc.profiles SET ... nik_encrypted = $3 ...
WHERE id = $1 AND nik_encrypted IS NULL
```
Plus return `rows_affected()` untuk signal ke service bahwa operasi gagal karena NIK sudah ada.
Atau alternatif: bungkus `find_by_auth_id` + `update_profile` + `create_submission` dalam **database transaction** (`pool.begin()`).

**Referensi CLAUDE.md:** §4.5 baris 179-181 — "operasi gabungan harus atomik — bungkus dalam transaction (`pool.begin()` … `commit()`), atau pola DELETE-RETURNING"

---

### C2. 🔴 Tidak Ada Database Transaction di `submit_kyc` — Partial Failure = User Terjebak

**File:** [service.rs:140-184](rust-services/user-service/src/application/service.rs#L140)
**Aturan CLAUDE.md dilanggar:** §4.5 — "pastikan setiap `begin()` berakhir `commit`/`rollback`"

**Masalah:**
`submit_kyc` melakukan 3 operasi tulis berurutan TANPA transaction boundary:
1. `update_profile` (menulis NIK_encrypted + data diri)
2. `create_submission` (membuat submission KYC)
3. `set_account_status` (via AuthClient — di luar transaction)

Jika langkah 1 sukses dan langkah 2 gagal (mis. constraint violation, connection drop):
- Profil sudah punya `nik_encrypted.is_some() = true`
- Tapi tidak ada submission
- User retry → ditolak dengan "NIK tidak dapat diubah" (guard line 147)
- **User terjebak permanen**, tidak bisa mengirim KYC ulang

**Failure Scenario:**
1. User submit KYC, `update_profile` sukses (NIK tersimpan).
2. `create_submission` gagal — DB connection terputus tepat setelah UPDATE.
3. Error dikembalikan ke client.
4. User retry → `find_by_auth_id` → `nik_encrypted.is_some() = true` → `Err("NIK tidak dapat diubah")`.
5. Tidak ada mekanisme pemulihan.

**Perbaikan:**
Bungkus `find_by_auth_id` + `update_profile` + `create_submission` dalam satu **PostgreSQL transaction**:

```rust
let mut tx = self.repo.begin().await?;
let mut profile = tx.find_by_auth_id(auth_id).await?;
// ... NIK check, encrypt, mutate profile ...
tx.update_profile(&profile).await?;
let submission = tx.create_submission(profile.id).await?;
// Transactional boundary end — baru panggil AuthClient (eksternal, tidak bisa dalam TX)
tx.commit().await?;
self.auth_client.set_account_status(...).await?; // best-effort setelah commit
```

> Catatan: `set_account_status` tidak bisa di-wrap dalam DB transaction karena beda service.
> Jika gagal setelah commit, submission sudah tercatat tapi status akun tetap `profile_incomplete`.
> Ini trade-off yang diterima (lebih baik punya submission + status salah, daripada NIK tersimpan tanpa submission).

**Referensi CLAUDE.md:** §4.5 baris 182-183 — "Zero Connection Leak: jangan tahan koneksi/transaction lewat `.await` yang panjang"

---

### C3. 🔴 `GET /{id}` Tanpa Autentikasi — IDOR / Information Disclosure

**File:** [mod.rs:98-100](rust-services/user-service/src/interface/mod.rs#L98)
**Aturan CLAUDE.md dilanggar:** §4.4 — "IDOR → 404, bukan 403. Ownership check di query" dan §4.4 — "3 lapis: autentikasi di middleware, ownership check di repository, visibility di handler"

**Masalah:**
Router mendaftarkan rute publik:
```rust
let public = Router::new()
    .route("/health", get(handlers::health))
    .route("/{id}", get(handlers::get_by_id))  // ← TANPA middleware auth!
    .with_state(state);
```

Handler `get_by_id` mengembalikan `UserProfileResponse` yang berisi:
- `phone` (nomor telepon)
- `full_name` (nama lengkap)
- `bio`
- `kyc_status` (status verifikasi)
- `nik_masked` (4 digit terakhir NIK)

Tanpa autentikasi, siapa pun bisa enumerasi UUID dan mendapatkan data pribadi pengguna.

**Failure Scenario:**
1. Attacker mendapat UUID dari log/referer/brute-force (`/api/v1/users/{uuid}`).
2. Memanggil tanpa token auth → 200 OK dengan data `phone`, `full_name`, `bio`, `kyc_status`.
3. Data ini cukup untuk social engineering, phishing tertarget, atau enumerasi pengguna.

**Perbaikan:**
Hapus rute publik, atau pindahkan ke rute terproteksi dengan ownership check:

```rust
// Di router protected:
.route("/{id}", get(handlers::get_by_id))
// Ownership check via claims.user_id
```

Atau jika endpoint ini memang diperlukan (mis. untuk service-to-service), tambahkan
middleware autentikasi internal/service.

**Referensi CLAUDE.md:** §4.4 baris 172 — "3 lapis: autentikasi di middleware, ownership check di repository, visibility di handler"

---

## Temuan HIGH

### H1. 🟠 NIK Validation Hanya Cek Length — Non-Digit Characters Lolos

**File:** [dto.rs:40-41](rust-services/user-service/src/application/dto.rs#L40)
**Aturan CLAUDE.md dilanggar:** §4.5 — "Validasi & sanitasi semua input I/O eksternal"

**Masalah:**
```rust
#[validate(length(min = 16, max = 16, message = "NIK harus 16 digit"))]
pub nik: String,
```
Validator `length(min = 16, max = 16)` hanya memeriksa panjang, bukan konten. String seperti
`"abcdefghijklmnop"` atau `"1111111111111111"` (16 karakter non-digit atau digit tidak valid)
lolos validasi. NIK Indonesia hanya terdiri dari digit numerik.

**Failure Scenario:**
1. User submit KYC dengan `nik: "ABCD1234EFGH5678"` — 16 karakter, lolos length check.
2. NIK dienkripsi dan disimpan. `nik_last4 = "5678"`. KYC pending.
3. Admin lihat `nik_masked = "xxx...5678"`, tidak tahu NIK asli tidak valid.
4. Masalah baru ketahuan saat perlu verifikasi Dukcapil/eKYC — NIK tidak ditemukan di database kependudukan.

**Perbaikan:**
Tambah validasi regex pada field `nik`:
```rust
#[validate(regex(path = *NIK_REGEX, message = "NIK harus 16 digit angka"))]
pub nik: String,
```
Atau validasi manual di `submit_kyc`:
```rust
if !input.nik.chars().all(|c| c.is_ascii_digit()) {
    return Err(anyhow::anyhow!("NIK harus berupa 16 digit angka"));
}
```

---

### H2. 🟠 `async-trait` pada `UserRepository` Tidak Diperlukan — Melanggar §4.1 CLAUDE.md

**File:** [repository.rs:47](rust-services/user-service/src/domain/repository.rs#L47)
**Aturan CLAUDE.md dilanggar:** §4.1 — "Native `async fn in trait` (Rust ≥ 1.75) — jangan tambah `async-trait` kecuali memang dibutuhkan untuk `dyn` trait object"

**Masalah:**
`UserRepository` menggunakan `#[async_trait::async_trait]` tetapi trait ini **tidak pernah** digunakan
sebagai `dyn UserRepository`. Verifikasi: `grep "dyn UserRepository"` → **0 hasil** di seluruh workspace.

`UserService<R: UserRepository>` menggunakan generic parameter, bukan trait object. Semua pemanggil
menggunakan tipe konkret `PgUserRepository`. `async-trait` di sini melanggar aturan eksplisit CLAUDE.md §4.1.

> **Catatan CLAUDE.md**: "jangan tambah `async-trait` kecuali memang dibutuhkan untuk `dyn` trait object
> (mis. `dyn AuthClient`); itu satu-satunya pengecualian."

`AuthClient`, `RegionClient`, `StorageClient`, `NotificationClient` menggunakan `async-trait` → benar,
karena dipakai sebagai `Arc<dyn AuthClient>` (trait object). Tapi `UserRepository` tidak.

**Perbaikan:**
Hapus `#[async_trait::async_trait]` dari `UserRepository` trait:
```rust
// Sebelum
#[async_trait::async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<UserProfile>, anyhow::Error>;
    ...
}

// Sesudah (native async fn in trait, Rust 1.75+)
pub trait UserRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<UserProfile>, anyhow::Error>;
    ...
}
```

Hapus juga `#[async_trait::async_trait]` dari `impl UserRepository for PgUserRepository` ([pg_repository.rs:86](rust-services/user-service/src/infrastructure/pg_repository.rs#L86)).

Rust 1.75+ mendukung native `async fn in trait`; Axum 0.8 + Tokio 1.x sudah fully compatible.
Kecuali trait dipakai via `dyn` (trait object), `async-trait` crate tidak diperlukan.

> **PENTING:** Cek kompatibilitas dengan Rust toolchain yang dipakai CI. Jika Rust < 1.75,
> maka exception ini ditangguhkan. Cek `rust-toolchain.toml` atau `Cargo.toml` `rust-version`.

---

### H3. 🟠 `log_upload_issued` Audit Error Discarded — Melanggar Q2 Audit Trail

**File:** [handlers.rs:159](rust-services/user-service/src/interface/handlers.rs#L159)
**Aturan CLAUDE.md dilanggar:** §4.5 — "I/O aman: degradasi anggun (mis. Redis down → DB tetap simpan, push di-skip dengan `warn!`)"

**Masalah:**
```rust
let _ = s
    .user_svc
    .log_upload_issued(claims.user_id, &perm.object_key, None)
    .await;
```

Error dari `log_upload_issued` (INSERT ke `document_access_log`) di-discard tanpa log.
Bandingkan dengan `commit_document` dan `get_document_url` yang mem-propagasi error audit via `?`.

Design Q2 mensyaratkan: **setiap akses dokumen wajib tercatat di audit trail** — `upload_issued`,
`commit`, `read_issued`. Silent discard di sini membuat audit upload tidak terlacak.

**Failure Scenario:**
1. `document_access_log` table temporarily unavailable (mis. DB connection pool exhausted).
2. User request presigned upload → URL diterbitkan.
3. Audit INSERT gagal → error di-discard.
4. Upload terjadi, tetapi tidak ada record `upload_issued`.
5. Saat audit/sengketa, rantai kepemilikan dokumen tidak utuh — celah hukum untuk UU PDP.

**Perbaikan:**
Minimal: log error. Ideal: propagasi error agar upload gagal total bila audit tidak bisa dicatat.

```rust
// Minimal — setidaknya log
if let Err(e) = s.user_svc.log_upload_issued(claims.user_id, &perm.object_key, None).await {
    tracing::warn!(user_id = %claims.user_id, error = %e, "gagal catat audit upload_issued");
}

// Ideal — gagalkan upload bila audit tidak bisa dicatat (integritas penuh)
s.user_svc.log_upload_issued(claims.user_id, &perm.object_key, None)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("gagal catat audit: {e}")))?;
```

---

## Temuan MEDIUM

### M1. 🟡 Cooldown KYC — Calendar Days vs Business Days (K16)

**File:** [service.rs:155-156](rust-services/user-service/src/application/service.rs#L155)
**Aturan CLAUDE.md dilanggar:** Tidak eksplisit, tapi melanggar business requirement K16

**Masalah:**
```rust
let cooldown_end = reviewed_at + chrono::Duration::days(KYC_COOLDOWN_BUSINESS_DAYS);
```

Konstanta bernama `KYC_COOLDOWN_BUSINESS_DAYS` = 3, tapi `chrono::Duration::days(3)` menghitung
72 jam kalender, bukan 3 **hari kerja** (Senin-Jumat). Rejection hari Jumat → cooldown berakhir
Senin (hanya 1 hari kerja), bukan Kamis (3 hari kerja sesungguhnya).

**Failure Scenario:**
1. User ditolak Jumat 2026-06-12. Dengan 72 jam, cooldown berakhir Senin 2026-06-15.
2. User bisa resubmit Senin siang — hanya melewati 1 hari kerja.
3. Tujuan K16 (cooling-off period 3 hari kerja untuk perbaikan data) tidak tercapai.

**Perbaikan:**
Ganti `chrono::Duration::days(3)` dengan logika hari kerja yang menghitung 3 Senin-Jumat:

```rust
fn add_business_days(date: DateTime<Utc>, days: i64) -> DateTime<Utc> {
    let mut result = date;
    let mut added = 0;
    while added < days {
        result = result + chrono::Duration::days(1);
        let weekday = result.weekday();
        if weekday != Weekday::Sat && weekday != Weekday::Sun {
            added += 1;
        }
    }
    result
}
```

Atau: rename konstanta ke `KYC_COOLDOWN_CALENDAR_DAYS` jika memang disengaja 3 hari kalender.
Yang penting: konsisten antara nama dan implementasi.

---

### M2. 🟡 Tidak Ada Unit Test di `user-service` Crate — Coverage 85% Terancam

**File:** (tidak ada file test di user-service crate)
**Aturan CLAUDE.md dilanggar:** §2 — "Menerapkan Unit Testing minimal overall coverage ≥ 85%" dan §4.8 — "Unit test murni inline `#[cfg(test)]`"

**Masalah:**
User-service crate (`rust-services/user-service/`) **tidak memiliki satu pun unit test**.
Glob `**/*test*` di user-service → 0 hasil. Semua test ada di `rejki-app/tests/` — integration test,
bukan unit test. Ini berarti:

1. Domain logic (entity, validation) — **0% covered**
2. Application service — **0% unit covered** (hanya di-cover integration test tidak langsung)
3. DTO validation — **0% covered**
4. Repository — **0% unit covered**

CLAUDE.md §4.8: "Unit test murni inline `#[cfg(test)]`; integration test di `tests/` pakai DB nyata"

**Perbaikan:**
Tambahkan unit test inline di tiap module. Minimal:

```rust
// src/domain/entity.rs — unit test
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kyc_status_from_str_valid() {
        assert_eq!("pending".parse::<KycSubmissionStatus>(), Ok(KycSubmissionStatus::Pending));
        assert_eq!("approved".parse::<KycSubmissionStatus>(), Ok(KycSubmissionStatus::Approved));
        assert_eq!("rejected".parse::<KycSubmissionStatus>(), Ok(KycSubmissionStatus::Rejected));
    }

    #[test]
    fn test_kyc_status_from_str_invalid() {
        assert!("invalid".parse::<KycSubmissionStatus>().is_err());
    }
}

// src/application/dto.rs — unit test validasi
#[cfg(test)]
mod tests {
    #[test]
    fn test_nik_rejects_non_digit() { ... }
    #[test]
    fn test_nik_rejects_short() { ... }
    #[test]
    fn test_nik_accepts_valid() { ... }
}
```

Ini penting untuk mencapai target **overall coverage ≥ 85%** sesuai Acceptance Criteria §2.

---

### M3. 🟡 CSV Injection (CWE-1236) — Formula Injection via `escape_csv`

**File:** [handlers.rs:344-349](rust-services/user-service/src/interface/handlers.rs#L344)
**Aturan CLAUDE.md dilanggar:** §2 — "Zero Security Issue"

**Masalah:**
`escape_csv` tidak mencegah **CSV formula injection** (CWE-1236). Karakter `=`, `+`, `-`, `@`
di awal cell dapat menyebabkan eksekusi formula saat CSV dibuka di Excel/LibreOffice Calc.

```rust
fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}
```

**Failure Scenario:**
1. User register dengan `full_name = "=cmd|'/C calc'!A0"` (atau varian: `+command`, `-DDE`, `@SUM`)
2. Admin export CSV → open di Excel.
3. Cell berisi karakter formula dieksekusi Excel — potensi arbitrary command execution.
4. Meski Excel modern memiliki warning, banyak user mengabaikannya.

**Perbaikan:**
Prefix cell yang diawali karakter formula dengan tab (`\t`) atau single quote (`'`):

```rust
fn escape_csv(s: &str) -> String {
    let escaped = if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    };
    // Cegah CSV formula injection (CWE-1236): prefix dengan tab
    if escaped.starts_with(['=', '+', '-', '@']) {
        format!("\t{escaped}")
    } else {
        escaped
    }
}
```

---

## Temuan LOW

### L1. ⚪ `warn_slow!` Tidak Diterapkan pada `update_profile` dan `update_avatar`

**File:** [pg_repository.rs:250-287](rust-services/user-service/src/infrastructure/pg_repository.rs#L250)
**Aturan CLAUDE.md dilanggar:** §4.6 — "`warn_slow!($start, $op)` di tiap method repository"

**Masalah:**
`update_profile` (14-kolom UPDATE, query termahal di repository) dan `update_avatar` tidak memiliki
`warn_slow!`, sementara 14 method repository lainnya memilikinya. Inkonsistensi observabilitas.

**Perbaikan:**
Tambahkan `let t = Instant::now();` di awal dan `warn_slow!(t, "...");` di akhir kedua method.

---

## Observasi Minor (Non-Blocking)

Berikut temuan dari angle D (reuse), E (simplification), F (efficiency), G (altitude), H (conventions)
yang diverifikasi sebagai **improvement, bukan bug** — dicatat untuk iterasi berikutnya:

| # | Deskripsi | Severity | Prioritas |
|---|-----------|----------|-----------|
| O1 | `find_by_auth_id` + `ok_or_else` pattern diulang 6× di service.rs — ekstrak helper `get_profile_by_auth_id()` | — | Low |
| O2 | 17-kolom SELECT list diulang di 7 SQL constant — gunakan `concat!()` macro | — | Low |
| O3 | `to_admin_list_item` dan `to_admin_detail` duplikasi 13 field mapping — `to_admin_detail` bisa reuse `to_admin_list_item` | — | Low |
| O4 | `escape_csv` mengalokasikan ulang meski input tanpa karakter khusus — return `Cow<str>` | — | Low |
| O5 | CSV export membangun seluruh string di memori (ADMIN_CSV_MAX = 10.000) — bisa pakai streaming | — | Low |
| O6 | `notify()` meng-clone `&str` ke `String` meski caller sudah punya owned `String` | — | Low |
| O7 | `get_profile` + `get_latest_submission` = 2 round-trip per profil view — bisa 1 query dengan LEFT JOIN | — | Medium |
| O8 | `admin_list_kyc` tidak forward `sort_by` query param ke service — field diterima tapi diabaikan | — | Low |

---

## Verifikasi terhadap CLAUDE.md & Acceptance Criteria

### Clean Architecture (§3) ✅ PASS

```
interface → application → domain ← infrastructure
   ✅           ✅           ✅           ✅
```

Layer terpisah, arah dependency satu arah, tidak ada pelanggaran.

### SOLID (§3, §4) ✅ PASS

| Prinsip | Status | Catatan |
|---------|--------|---------|
| **S** | ✅ | Service, Repository, Handler — tanggung jawab terpisah |
| **O** | ✅ | Trait-based contracts |
| **L** | ✅ | `PgUserRepository` substitusi `UserRepository` |
| **I** | ✅ | `UserRepository` 17 method fokus, `UserClient` 3 method minimal |
| **D** | ✅ | `UserService<R: UserRepository>` — generic atas trait |

> **Catatan:** `UserRepository` via generic, bukan `dyn` → ini benar per CLAUDE.md §3: "Generic atas trait repo (`Service<R: Repository>`), bukan `Box<dyn Trait>` (hindari vtable overhead, wiring terverifikasi compile-time)."

Tapi karena bukan `dyn`, `async-trait` di `UserRepository` tidak diperlukan (lihat H2).

### Boundary Arsitektur (§3) ✅ PASS

| Aturan | Status |
|--------|--------|
| domain tidak import infrastructure/interface | ✅ |
| Wiring konkret di `router()` | ✅ |
| `rejki-app` tidak import `*-service-client` | ✅ |
| Komunikasi antar-service via `*-service-client` trait | ✅ |
| Satu schema PostgreSQL per service (`user_svc`) | ✅ |
| Tidak ada cross-schema FK/JOIN | ✅ |

### Rust + Axum (§4)

| Aturan | Status | Catatan |
|--------|--------|---------|
| §4.1 Versi dipin di workspace root | ✅ | Semua `{ workspace = true }` |
| §4.1 Native `async fn in trait` | ⚠️ | `UserRepository` pakai `async-trait` tapi bukan `dyn` (H2) |
| §4.2 Error handling `AppError` + `ApiResponse<T>` | ✅ | `map_review_err`, `map_storage_err` |
| §4.2 POST 201 + Location, DELETE 204 | ✅ | `submit_kyc` → 201 |
| §4.3 Schema per service, kolom mandatory | ✅ | `id, created_at, updated_at` |
| §4.3 Index FK-equivalent | ✅ | `idx_profiles_auth_id`, `idx_kyc_submission_*` |
| §4.3 `updated_at` eksplisit di query | ✅ | `SET updated_at = now()` |
| §4.3 Parameterized query 100% | ✅ | `sqlx::query!` / `bind()` |
| §4.4 IDOR → 404 | ✅ | `admin_get_document` → 404 |
| §4.4 Ownership check di query | ⚠️ | `update_profile` tidak guarded (C1) |
| §4.5 Transaction untuk operasi gabungan | 🔴 | Tidak ada transaction di `submit_kyc` (C2) |
| §4.5 Zero Race Condition | 🔴 | `update_profile` tanpa guard atomik (C1) |
| §4.5 Zero Connection Leak | ✅ | `PgPool` managed by sqlx |
| §4.5 Retry aman + exponential backoff | 🔶 | Tidak ada mekanisme retry di user-service |
| §4.6 `warn_slow!` di tiap repository method | ⚠️ | `update_profile`, `update_avatar` belum (L1) |
| §4.7 Zero Too Many Arguments | ✅ | `AdminKycListParams` struct, `UserService::new` 5 params (DI) |
| §4.7 Zero God Function | ✅ | Method terbesar ~96 baris (`submit_kyc`) |
| §4.7 Zero God Class | ✅ | `UserService` ~500 baris, `PgUserRepository` ~412 baris |
| §4.8 Unit test `#[cfg(test)]` | 🔴 | **Tidak ada unit test** di user-service crate (M2) |
| §4.8 Naming test standar | ✅ | Integration test: `test_{unit}_given_{cond}_when_{act}_then_{exp}` |
| §4.9 `cargo fmt` + `cargo clippy` | ⚠️ | Perlu verifikasi |
| §5 Axum 0.8, handler async | ✅ | `State<T>`, `Extension<AuthClaims>` |
| §5 Graceful shutdown | ✅ | Di `rejki-app` |
| §5 Swagger dev-only | ✅ | `#[cfg(feature = "swagger")]` di `rejki-app` |

### "Zero …" Checklist (§2)

| Kriteria | Status | Temuan |
|----------|--------|--------|
| Zero Race Condition | 🔴 | C1 — `update_profile` tanpa guard atomik |
| Zero N+1 Query | ✅ | `COUNT(*) OVER()` satu round-trip, `LEFT JOIN` di admin listing |
| Zero Connection Leak | ✅ | `PgPool` managed by sqlx |
| Zero Hardcoded | ✅ | Konstanta bernama (`KYC_COOLDOWN_BUSINESS_DAYS`, `ADMIN_LIST_DEFAULT_LIMIT`, dll.) |
| Zero God Function | ✅ | Tidak ada method >100 baris |
| Zero God Class | ✅ | `UserService` ~500 baris + helper |
| Zero Security Issue | ⚠️ | C3 (IDOR), M3 (CSV injection) |
| Zero Circular Dependency | ✅ | user-service → clients (auth, storage, region, notification) — tidak ada balik |
| Zero Too Many Arguments (Rust) | ✅ | `AdminKycListParams`, `CommitDocumentInput` — struct-based |

---

## Rekomendasi Perbaikan Prioritas

### Immediate (CRITICAL — sebelum production)

1. **C1** — Tambah `AND nik_encrypted IS NULL` guard di `update_profile` atau wrap dalam transaction
2. **C2** — Wrap `submit_kyc` write operations dalam database transaction
3. **C3** — Hapus rute `GET /{id}` dari public router; pindah ke protected + ownership check

### Short-term (HIGH — iterasi berikutnya)

4. **H1** — Tambah validasi digit-only untuk NIK
5. **H2** — Hapus `async-trait` dari `UserRepository` (native async)
6. **H3** — Propagasi atau minimal log error dari `log_upload_issued`

### Medium-term (MEDIUM)

7. **M1** — Ganti cooldown 3 hari kalender → 3 hari kerja, atau rename konstanta
8. **M2** — Tambah unit test di user-service crate (kejar coverage 85%)
9. **M3** — Tambah CSV formula injection protection

### When available (LOW)

10. **L1** — Tambah `warn_slow!` ke `update_profile` dan `update_avatar`

---

## Daftar File yang Perlu Dimodifikasi

| File | Temuan | Perubahan |
|------|--------|-----------|
| `infrastructure/pg_repository.rs` | C1, L1 | Guard atomik + `warn_slow!` |
| `application/service.rs` | C2, M1, H2 | Transaction, cooldown business days |
| `interface/mod.rs` | C3 | Hapus public GET /{id} |
| `application/dto.rs` | H1 | Validasi regex NIK |
| `interface/handlers.rs` | H3, M3 | Propagasi audit error, CSV injection fix |
| `domain/repository.rs` | H2 | Hapus `async-trait` |
| `infrastructure/pg_repository.rs` | H2 | Hapus `async-trait` |
| `Cargo.toml` | H2 | Remove `async-trait` dep (jika tidak dipakai lain) |
| `src/**/*.rs` (test) | M2 | Tambah unit test inline |

---

## OpenSpec Tracking

Temuan code review ini perlu dilacak sebagai OpenSpec change baru atau task di change existing.
Berdasarkan CLAUDE.md §8: "Perubahan dikelola spec-driven. Sebelum koding fitur baru: ada proposal + spec + tasks."

**Rekomendasi:** Buat OpenSpec change `fix-user-service-code-review-findings` (mengikuti pola
`fix-auth-service-code-review-findings` yang sudah ada).

### Task yang perlu dibuat:

- [ ] **C1 — Guard atomik NIK di `update_profile`**: Tambah `AND nik_encrypted IS NULL` + test unit
- [ ] **C2 — Transaction `submit_kyc`**: Wrap write ops dalam `pool.begin()` + test integration (retry pada partial failure)
- [ ] **C3 — Hapus public GET `/{id}`**: Pindah ke protected router dengan ownership check
- [ ] **H1 — Validasi digit NIK**: Tambah regex/digit check + unit test
- [ ] **H2 — Hapus `async-trait` dari `UserRepository`**: Native async fn in trait + verifikasi kompatibilitas Rust toolchain
- [ ] **H3 — Propagasi audit error**: Ubah `let _ =` jadi `?` atau minimal `tracing::warn!`
- [ ] **M1 — Cooldown 3 hari kerja**: Ganti dari kalender ke bisnis, atau rename konstanta
- [ ] **M2 — Unit test user-service**: Tambah `#[cfg(test)] mod tests` di tiap module, target coverage ≥ 85%
- [ ] **M3 — CSV injection protection**: Tambah prefix `\t` untuk cell berawalan `= + - @`
- [ ] **L1 — `warn_slow!` `update_profile` + `update_avatar`**: Tambah monitoring

---

## Verifikasi Final

```
cargo check --workspace                    → TBD (setelah fix)
cargo clippy --workspace -- -D warnings    → TBD (setelah fix)
cargo fmt --check                          → TBD (setelah fix)
cargo test --workspace                     → TBD (setelah tambah unit test)
cargo sqlx prepare --workspace             → TBD (bila ada query baru)
```

---

**Kesimpulan:** User-service secara arsitektur sudah baik (Clean Architecture, SOLID, boundary bersih).
Tiga temuan CRITICAL harus difix sebelum production deployment. Temuan HIGH dan MEDIUM harus difix
dalam iterasi berikutnya. Tidak ada unit test di user-service crate — ini gap terbesar untuk
mencapai target coverage 85%.

**File laporan ini:** `docs/code-review/code-review-user-service-jun-2026.md`
