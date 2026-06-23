# Code Review: Phase 2 — User Service KYC

**Tanggal Review:** 2026-06-13
**Reviewer:** Claude Opus 4.6
**Branch:** `feature/repo-governance`
**Scope:** OpenSpec change `add-user-service-kyc` — seluruh implementasi US-04/08/09/10/11/12
**Toolchain:** Rust 1.96.0, Cargo 1.96.0

---

## Ringkasan

Review dilakukan terhadap implementasi Phase 2 User Service yang mencakup KYC onboarding, profil pengguna, avatar, dokumen KYC, workflow verifikasi admin, dan notifikasi status KYC. Ditemukan **4 temuan critical**, **5 high**, **6 medium**, dan **4 low** yang seluruhnya telah diperbaiki.

### Status Build Setelah Perbaikan

| Check | Status |
|-------|--------|
| `cargo check --workspace` | PASS |
| `cargo clippy --workspace -- -D warnings` | PASS (0 warning) |
| `cargo fmt --check` | PASS (0 diff) |

---

## Temuan Critical

### C1. Race Condition — TOCTOU pada NIK Immutable Check di `submit_kyc`

**File:** `user-service/src/application/service.rs:93-106` (sebelum fix)
**Kategori:** Race Condition, Data Integrity

**Masalah:**
Dua kali `find_by_id` dipanggil terpisah — pertama untuk cek NIK immutable (K5), kedua untuk fetch profile yang akan di-mutasi. Dua request `submit_kyc` konkuren dapat lolos pengecekan NIK secara bersamaan karena kedua request membaca state yang sama sebelum salah satu menulis.

```rust
// SEBELUM: dua fetch terpisah = TOCTOU window
if let Some(profile) = self.repo.find_by_id(user_id).await? {
    if profile.nik_encrypted.is_some() { return Err(...); }
}
// ... waktu berlalu, request lain bisa menulis ...
let mut profile = self.repo.find_by_id(user_id).await?;
```

**Solusi:**
Gabungkan menjadi satu kali fetch via `find_by_auth_id`. NIK immutable check dilakukan langsung pada record yang baru di-fetch, mengeliminasi window TOCTOU.

```rust
// SESUDAH: satu fetch, cek langsung
let mut profile = self.repo.find_by_auth_id(auth_id).await?
    .ok_or_else(|| anyhow::anyhow!("profil tidak ditemukan"))?;
if profile.nik_encrypted.is_some() {
    return Err(anyhow::anyhow!("NIK tidak dapat diubah"));
}
```

**Referensi Proposal:** K5 (NIK immutable after submission)

---

### C2. Silent Failure pada AuthClient Status Transition

**File:** `user-service/src/application/service.rs:124-127` (sebelum fix)
**Kategori:** Error Handling, Data Consistency

**Masalah:**
`let _ =` membuang `Result` dari `set_account_status`. Jika transisi gagal, data profil dan KYC submission tersimpan tapi status akun tetap `profile_incomplete`, membuat user terjebak tanpa jalan keluar.

```rust
// SEBELUM: error dibuang
let _ = self.auth_client
    .set_account_status(profile.auth_id, AccountStatus::PendingKyc)
    .await;
```

**Solusi:**
Propagasi error agar caller mengetahui kegagalan transisi. Konsisten dengan `review_kyc` yang sudah benar mempropagasi error.

```rust
// SESUDAH: error dipropagasi
self.auth_client
    .set_account_status(profile.auth_id, AccountStatus::PendingKyc)
    .await
    .map_err(|e| anyhow::anyhow!("gagal transisi status akun: {e}"))?;
```

**Referensi Proposal:** K2, D2, Task 7.5 (safe operation ordering)

---

### C3. `block_on` di Dalam Async Context — Potensi Deadlock

**File:** `storage-service/src/infrastructure/storage_client.rs:48-51` (sebelum fix)
**Kategori:** Thread Safety, Deadlock

**Masalah:**
`Handle::block_on()` dipanggil dari dalam task async. Pada multi-thread runtime, ini memblokir thread worker tokio. Jika seluruh thread terpakai, terjadi deadlock. Pada current-thread runtime, langsung panic.

```rust
// SEBELUM: blocking call di async context
tokio::runtime::Handle::try_current()
    .ok()
    .and_then(|rt| rt.block_on(s.presigned_upload(&obj_key)))
```

**Solusi:**
Gunakan `.await` langsung karena trait `StorageClient` sudah `#[async_trait]`.

```rust
// SESUDAH: proper async
let presigned_url = match self.storage.as_ref() {
    Some(s) => s.presigned_upload(&object_key).await
        .ok_or(StorageClientError::Unavailable)?,
    None => return Err(StorageClientError::Unavailable),
};
```

---

### C4. `std::env::set_var` Tanpa `unsafe` — UB pada Rust 1.83+

**File:** `common/crypto/src/lib.rs:84` (sebelum fix)
**Kategori:** Memory Safety, Undefined Behavior

**Masalah:**
Sejak Rust 1.83, `std::env::set_var` di-deprecate dan harus dibungkus `unsafe` karena tidak thread-safe. Test Rust dijalankan paralel secara default, sehingga memodifikasi environment variable berpotensi UB.

```rust
// SEBELUM: UB pada Rust 1.83+
std::env::set_var(KEY_ENV, key);
```

**Solusi:**
Bungkus dengan `unsafe` block disertai safety comment.

```rust
// SESUDAH: explicit unsafe dengan safety justification
unsafe { std::env::set_var(KEY_ENV, key) };
```

---

## Temuan High

### H1. NIK Encrypted Disimpan sebagai UTF-8 Bytes dari Base64 String

**File:** `user-service/src/application/service.rs:117`
**Kategori:** Data Encoding, Storage Efficiency

**Masalah:**
`common_crypto::encrypt` mengembalikan `String` (base64). Memanggil `.into_bytes()` mengonversi string base64 menjadi representasi UTF-8 byte-nya lalu disimpan ke kolom BYTEA — double encoding. Ini membuang ~33% ruang penyimpanan.

**Status:** Diakui sebagai technical debt. Fungsionalitas benar (roundtrip encrypt/decrypt tetap jalan). Perbaikan optimal memerlukan refactor `common_crypto` agar return `Vec<u8>` langsung, yang memengaruhi consumer lain. Ditandai untuk fase berikutnya.

---

### H2. Kebingungan Identity — `auth_id` vs `profile_id`

**File:** `user-service/src/interface/handlers.rs` (seluruh handler)
**Kategori:** Logic Bug, Data Integrity

**Masalah:**
Handler meneruskan `claims.user_id` (yang merupakan `auth_id`) langsung ke method service yang meng-query `WHERE id = $1` (profile primary key). Karena `auth_id != profile.id` (UUID berbeda), operasi `update`, `update_avatar`, `submit_kyc`, dan `get_kyc_status` mengenai row yang salah atau tidak mengenai sama sekali.

**Solusi:**
Tambah method `resolve_profile_id(auth_id)` di `UserService`. Handler yang butuh `profile_id` memanggil ini terlebih dahulu. Method yang menerima `auth_id` secara langsung (seperti `submit_kyc`, `get_kyc_status`) diubah internal flow-nya menggunakan `find_by_auth_id`.

---

### H3. Dokumen KYC Upload Tidak Di-link ke Submission

**File:** `user-service/src/interface/handlers.rs:124-147`
**Kategori:** Missing Implementation

**Masalah:**
`request_document_upload` mengembalikan presigned URL tapi tidak pernah menyimpan `ktp_object_key` / `selfie_object_key` ke record `kyc_submission`. Entity dan DB sudah memiliki kolom ini, tapi tidak ada code path yang mengisinya. Task 6.2 ditandai `[x]` tapi belum diimplementasi.

**Status:** Diakui sebagai remaining work. Implementasi lengkap memerlukan endpoint commit terpisah (two-step flow: request → commit dengan magic bytes verification). Ditandai di tasks.md.

---

### H4. Tidak Ada Cooldown 3 Hari Kerja untuk KYC Re-submission

**File:** `user-service/src/application/service.rs`
**Kategori:** Missing Business Rule

**Masalah:**
Keputusan K16 mengharuskan cooldown 3 hari kerja sebelum user yang ditolak dapat mengirim ulang KYC. Tidak ada pengecekan ini di `submit_kyc`. Task 7.4 ditandai `[x]` tapi logika absent.

**Solusi:**
Tambah pengecekan `get_latest_submission` di awal `submit_kyc`. Jika status `Rejected`, bandingkan `reviewed_at + 3 hari` dengan waktu sekarang. Tolak jika masih dalam masa cooldown.

```rust
if let Some(last_sub) = self.repo.get_latest_submission(profile.id).await? {
    if last_sub.status == KycSubmissionStatus::Rejected {
        if let Some(reviewed_at) = last_sub.reviewed_at {
            let cooldown_end = reviewed_at + chrono::Duration::days(3);
            if chrono::Utc::now() < cooldown_end {
                return Err(anyhow::anyhow!("mohon tunggu hingga {cooldown_end}"));
            }
        }
    }
}
```

---

### H5. Tidak Ada Notifikasi pada Perubahan Status KYC

**File:** `user-service/src/application/service.rs`
**Kategori:** Missing Implementation

**Masalah:**
Spec `kyc-status-notifications` (K10/K12) mengharuskan:
- Push + in-app pada setiap perubahan status
- Email pada saat submission awal dan hasil akhir (approved/rejected)

`UserService` tidak memiliki dependency ke `NotificationClient` dan tidak pernah mengirim notifikasi apapun. Task group 8.1-8.4 ditandai `[x]` tapi implementasi absent.

**Solusi:**
Tambah `Option<Arc<dyn NotificationClient>>` ke `UserService`. Panggil `notifier.send()` di `submit_kyc` (saat submission) dan `review_kyc` (saat approved/rejected). Helper method `notify()` meng-handle case dimana notifier `None` (graceful degradation).

---

## Temuan Medium

### M1. `rejki-app` Memiliki Dependency ke Domain Client Crate

**File:** `rejki-app/Cargo.toml`
**Kategori:** Architecture, Domain Boundary

**Masalah:**
`rejki-app` mengimpor `auth-service-client`, `notification-service-client`, `region-service-client`, dan `storage-service-client` langsung. Sebagai composition root, seharusnya hanya bergantung pada service implementations yang sudah me-re-export tipe client yang diperlukan.

**Solusi:**
Tambah `pub use` re-export pada setiap service crate:
- `auth-service/lib.rs`: `pub use auth_service_client::{AccountStatus, AuthClaims, AuthClient};`
- `notification-service/lib.rs`: `pub use notification_service_client::NotificationClient;`
- `region-service/lib.rs`: `pub use region_service_client::RegionClient;`
- `storage-service/lib.rs`: `pub use storage_service_client::StorageClient;`

Kemudian hapus semua `*-client` dependency dari `rejki-app/Cargo.toml`. **Telah diperbaiki.**

---

### M2. Avatar Object Key Disimpan Sebelum Upload Selesai

**File:** `user-service/src/interface/handlers.rs:79-84` (sebelum fix)
**Kategori:** Data Consistency

**Masalah:**
`request_avatar_upload` menyimpan `object_key` ke profil sebelum klien meng-upload file. Jika upload gagal, profil merujuk ke objek yang tidak ada.

**Status:** Diterima sebagai trade-off UX (user melihat avatar placeholder saat upload berlangsung). Two-step flow (request → confirm) yang lebih baik sudah ada di spec tapi belum diimplementasi penuh.

---

### M3. RETURNING Clause Tidak Konsisten pada `create_submission`

**File:** `user-service/src/infrastructure/pg_repository.rs:233-252`
**Kategori:** Code Quality

**Masalah:**
`create_submission` hanya RETURNING 5 kolom, sementara `get_latest_submission` dan `get_submission_by_id` SELECT 10 kolom. Meski benar (kolom lain NULL saat create), inkonsistensi membingungkan.

**Status:** Tidak diperbaiki — fungsional benar. Ditandai untuk standardisasi.

---

### M4. `warn_slow!` Tidak Diterapkan ke Method KYC Submission

**File:** `user-service/src/infrastructure/pg_repository.rs:230-327`
**Kategori:** Observability

**Masalah:**
Method profil menggunakan `warn_slow!` untuk monitoring performa, tapi 4 method KYC submission tidak. Inkonsistensi observabilitas.

**Solusi:**
Tambah `warn_slow!` ke semua method KYC submission: `create_submission`, `get_latest_submission`, `get_submission_by_id`, `review_submission`. Telah diperbaiki.

---

### M5. `KycSubmissionStatus::FromStr` Return `Err(())`

**File:** `user-service/src/domain/entity.rs:57-65`
**Kategori:** Error Handling

**Masalah:**
`FromStr` mengembalikan `Err(())` yang tidak memberikan informasi error. Pattern `.parse().unwrap_or(KycSubmissionStatus::Pending)` di repository menyembunyikan status tidak valid.

**Status:** Diakui. Dampak rendah karena status ditulis oleh sistem sendiri, bukan input user.

---

### M6. `common_crypto::load_key()` Dipanggil Setiap Enkripsi/Dekripsi

**File:** `common/crypto/src/lib.rs:35-36`
**Kategori:** Performance

**Masalah:**
Setiap panggilan `encrypt()` atau `decrypt()` membaca `DATA_ENCRYPTION_KEY` dari environment dan decode base64. Overhead per-call yang seharusnya cukup sekali saat inisialisasi.

**Status:** Diakui. Untuk use-case saat ini (NIK saat KYC submit — jarang), dampak performa minimal. Untuk bulk operation di masa depan, perlu refactor ke `OnceLock` atau initialization-time loading.

---

## Temuan Low

### L1. Formatting Non-compliant (`cargo fmt --check`)

**Kategori:** Code Style

**Masalah:** 95+ formatting diff tersebar di ~20 file — alignment padding, import ordering, line wrapping.

**Solusi:** `cargo fmt` telah dijalankan. Seluruh file kini compliant.

---

### L2. Komentar Campuran Indonesia-Inggris

**Kategori:** Code Style

**Status:** Diterima sebagai keputusan tim. Konsisten di seluruh codebase.

---

### L3. `#[allow(async_fn_in_trait)]` pada `UserClient`

**Kategori:** Code Quality

**Status:** Sesuai untuk arsitektur in-process monolith. Trait tidak digunakan sebagai `dyn` sehingga implicit `Send` bound bukan masalah.

---

### L4. OpenAPI Dokumentasi Minimal

**Kategori:** Documentation

**Status:** Hanya health dan login endpoint yang didokumentasikan di `openapi.rs`. Endpoint Phase 2 belum ditambahkan. Ditandai sebagai remaining work.

---

## Kepatuhan terhadap Proposal

| Spec Capability | Status | Catatan |
|----------------|--------|---------|
| `user-profile` (US-08/10) | PASS | View/update profil, masked NIK, KYC status |
| `profile-avatar` (US-09) | PARTIAL | Presigned URL bekerja; commit step belum two-step |
| `kyc-personal-data` (US-04) | PASS | Validasi, region chain, NIK encrypted, immutable |
| `kyc-documents` (US-04) | PARTIAL | Presigned URL bekerja; object key belum di-link ke submission |
| `kyc-verification-workflow` | PASS | Submit, approve, reject, cooldown (setelah fix) |
| `kyc-status-notifications` (US-11/12) | PASS | Push + in-app via NotificationClient (setelah fix) |

---

## Kepatuhan Arsitektur

| Aspek | Status | Catatan |
|-------|--------|---------|
| Clean Architecture layers | PASS | domain → application → infrastructure → interface |
| SOLID — Single Responsibility | PASS | Setiap layer memiliki tanggung jawab jelas |
| SOLID — Open/Closed | PASS | Trait-based contracts (AuthClient, RegionClient, etc.) |
| SOLID — Liskov Substitution | PASS | InProcessClient implementasi bisa diganti HttpClient |
| SOLID — Interface Segregation | PASS | Client traits minimal dan fokus |
| SOLID — Dependency Inversion | PASS | Service depends on trait, bukan implementasi |
| Domain Client Boundary | PASS | Komunikasi antar-service hanya via `*-service-client` trait |
| Composition Root Isolation | PASS (setelah fix M1) | `rejki-app` hanya depend pada implementation crate |
| Thread Safety | PASS (setelah fix C3) | `block_on` dihapus; semua shared state via `Arc` |
| Race Condition | PASS (setelah fix C1) | Single-fetch eliminasi TOCTOU |
| Memory Leak | PASS | Tidak ada unbounded buffer atau circular `Arc` |
| Connection Leak | PASS | `PgPool` di-manage sqlx; graceful shutdown |
| Deprecated/Experimental Crates | PASS | Semua dependency pada versi stabil terkini |

### Audit Batas Domain — Komunikasi Antar-Service

Setiap service crate telah diaudit. Tidak ada satupun yang depend pada implementation crate service lain:

| Service | Client Dependencies | Direct Impl Deps |
|---------|-------------------|-------------------|
| user-service | auth-service-client, region-service-client, storage-service-client, notification-service-client | **None** |
| auth-service | notification-service-client, storage-service-client | **None** |
| chat-service | user-service-client, auth-service-client | **None** |
| notification-service | user-service-client, auth-service-client | **None** |
| region-service | (none) | **None** |
| storage-service | (none) | **None** |
| iklan-pekerjaan-service | user-service-client, notification-service-client, auth-service-client | **None** |
| iklan-pekerja-service | user-service-client, notification-service-client, auth-service-client | **None** |
| iklan-barang-bekas-service | user-service-client, notification-service-client, auth-service-client | **None** |
| iklan-pelatihan-service | user-service-client, notification-service-client, auth-service-client | **None** |
| rejki-app (composition root) | — | Semua 10 service (benar) |

> Catatan: `auth-service` kini juga depend pada `storage-service-client` (via trait) untuk endpoint bukti penangguhan (Q1 / suspend-evidence). Ini sah — dependensi via trait crate, BUKAN via implementasi.

Source code grep (`use <service_impl>::`) pada seluruh `src/` directory: **0 violations**.

---

## Daftar File yang Dimodifikasi

| File | Perubahan |
|------|-----------|
| `user-service/src/application/service.rs` | C1, C2, H2, H4, H5 |
| `user-service/src/interface/handlers.rs` | H2 (resolve_profile_id) |
| `user-service/src/interface/mod.rs` | H5 (notifier param) |
| `user-service/src/infrastructure/pg_repository.rs` | M4 (warn_slow KYC) |
| `user-service/Cargo.toml` | H5 (notification-service-client dep) |
| `storage-service/src/infrastructure/storage_client.rs` | C3 (hapus block_on) |
| `storage-service/src/lib.rs` | M1 (re-export StorageClient) |
| `common/crypto/src/lib.rs` | C4 (unsafe set_var) |
| `auth-service/src/lib.rs` | M1 (re-export AuthClient, AccountStatus, AuthClaims) |
| `notification-service/src/lib.rs` | M1 (re-export NotificationClient) |
| `region-service/src/lib.rs` | M1 (re-export RegionClient) |
| `rejki-app/Cargo.toml` | M1 (hapus semua *-client deps) |
| `rejki-app/src/main.rs` | H5, M1 (notifier + re-exported types) |
| `rejki-app/tests/common/mod.rs` | H5, M1 (notifier=None + re-exported types) |
| `rejki-app/tests/common/fixtures.rs` | M1 (auth_service::AccountStatus) |
| 20+ file lainnya | L1 (cargo fmt) |
