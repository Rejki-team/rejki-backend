## Context

User-service sudah memiliki domain KYC penuh (profil, submission, dokumen, audit, notifikasi) dari:
- `add-user-service-kyc` — flow onboarding, encrypted NIK, magic bytes, region validation, notifikasi
- `add-user-admin-management` — admin listing, detail, document access, CSV export, auto-purge, idempotensi review

Code review Juni 2026 (`docs/code-review/code-review-user-service-jun-2026.md`) menemukan 10 temuan
yang perlu diperbaiki. Change ini bersifat **korektif** pada service yang sudah ada — tanpa crate baru,
tanpa schema baru, tanpa perubahan kontrak API eksternal.

## Goals / Non-Goals

**Goals:**
- Eliminasi race condition pada NIK write + partial failure di `submit_kyc`
- Tutup celah IDOR pada public endpoint
- Perkuat validasi NIK (digit-only check)
- Hapus `async-trait` pada `UserRepository` (native async)
- Propagasi audit trail error (compliance Q2)
- Selaraskan cooldown dengan K16 (business days, atau rename konstanta)
- Cegah CSV formula injection (CWE-1236)
- Tambah unit test coverage ke ≥ 85%
- Lengkapi `warn_slow!` pada 2 method repository yang belum

**Non-Goals:**
- Tidak menambah fitur baru
- Tidak mengubah skema database
- Tidak mengubah kontrak API publik (kecuali menghapus endpoint publik yang seharusnya tidak publik)

## Decisions

### D1 — Transaction wrapping untuk `submit_kyc`
Wrap `find_by_auth_id` + `update_profile` + `create_submission` dalam satu PostgreSQL transaction
(`pool.begin()`). `set_account_status` (AuthClient) tetap di luar transaction karena cross-service
(bukan DB scope). Jika `set_account_status` gagal setelah commit, submission sudah tercatat —
trade-off diterima (lebih baik punya submission + status salah daripada NIK tersimpan tanpa submission).

### D2 — Guard NIK atomik di level database
`update_profile` akan menambahkan `AND nik_encrypted IS NULL` pada WHERE clause. Return
`rows_affected()` untuk signal ke service apakah NIK sudah ada (overwrite attempt). Ini lapis
kedua setelah guard aplikasi (single fetch `find_by_auth_id`).

### D3 — Hapus public `GET /{id}`; pindahkan ke protected
Endpoint `GET /api/v1/users/{id}` saat ini di public router (tanpa auth middleware). Data yang
dikembalikan mencakup `phone`, `full_name`, `bio`, `kyc_status` — informasi pribadi yang seharusnya
hanya bisa diakses pemilik atau admin. Pindahkan ke protected router dengan ownership check.

### D4 — Validasi NIK digit-only di input layer
Tambah regex `^[0-9]{16}$` atau manual `chars().all(|c| c.is_ascii_digit())` di `submit_kyc`
atau DTO validation. Validasi di layer paling awal menolak input non-digit sebelum enkripsi.

### D5 — Hapus `async-trait` dari `UserRepository`
`UserRepository` TIDAK pernah digunakan sebagai `dyn UserRepository` (verifikasi: grep 0 hasil).
`UserService<R: UserRepository>` menggunakan generic, bukan trait object. CLAUDE.md §4.1:
"jangan tambah `async-trait` kecuali memang dibutuhkan untuk `dyn` trait object."

### D6 — Cooldown: rename konstanta
Alih-alih mengimplementasi logika hari kerja (kompleks, butuh kalender hari libur), rename
konstanta `KYC_COOLDOWN_BUSINESS_DAYS` → `KYC_COOLDOWN_DAYS` dan dokumentasikan bahwa
ini adalah 3 hari kalender, bukan 3 hari kerja. Jika bisnis membutuhkan 3 hari kerja sesungguhnya,
implementasi kompleks dapat ditambahkan di change terpisah.

### D7 — CSV formula injection via prefix `\t`
Prefix setiap cell CSV yang diawali karakter formula (`= + - @`) dengan tab (`\t`) untuk
mencegah eksekusi formula di Excel/LibreOffice Calc (CWE-1236).

### D8 — Propagasi audit `upload_issued` error
Ganti `let _ =` dengan `?` di `request_document_upload` handler agar kegagalan pencatatan
audit menggagalkan seluruh operasi upload. Ini memastikan integritas audit trail Q2.

## Risks / Trade-offs

- **Transaction di `submit_kyc`**: Menahan koneksi DB lebih lama (~10-30ms tambahan). Risiko
  connection pool exhaustion bila volume KYC submission tinggi. Mitigasi: `set_account_status`
  tetap di luar transaction (non-blocking untuk auth service).
- **Public endpoint removal**: Jika ada consumer internal yang bergantung pada `GET /api/v1/users/{id}`
  publik, endpoint tidak bisa diakses tanpa token. Consumer perlu di-update untuk menyertakan auth token.
- **Cooldown rename**: Tidak mengubah perilaku; hanya mengklarifikasi bahwa ini 3 hari kalender,
  bukan 3 hari kerja. Jika bisnis menginginkan 3 hari kerja sesungguhnya, perlu implementasi terpisah.

## Migration Plan

1. Tambah guard atomik di `update_profile` (1 query change, backward-compatible)
2. Wrap `submit_kyc` dalam transaction (2 method repository baru: `begin`, atau refactor existing)
3. Hapus endpoint `GET /{id}` dari public router
4. Tambah validasi NIK di DTO atau service
5. Hapus `async-trait` dari `UserRepository` trait + impl
6. Propagasi audit error di handler
7. Rename konstanta cooldown
8. Update `escape_csv` untuk formula injection protection
9. Tambah `warn_slow!` ke `update_profile` + `update_avatar`
10. Tambah unit test di semua module user-service
11. Jalankan integration test existing → pastikan 13/13 PASS (0 regresi)
12. Update Swagger (development) untuk endpoint yang berubah

## Resolved Questions

- **Q: Perlu bulkhead/timeout untuk transaction `submit_kyc`?** Tidak — operasi single row,
  tidak ada long-held lock. `statement_timeout` sudah di-set di pool config.
- **Q: Apakah `dependency injection` ke `UserRepository` perlu diubah ke `dyn` setelah hapus `async-trait`?** Tidak — tetap pakai generic `R: UserRepository`. Native `async fn in trait`
  mendukung generic bounds tanpa `async-trait`.
- **Q: Apakah `update_profile` transaction akan blocking operasi lain?** Tidak — `UPDATE ...
  WHERE id = $1` lock on row level; concurrent `submit_kyc` untuk user yang sama akan antri
  (serialized), yang justru mencegah race condition. Untuk user berbeda, tidak ada blocking.

## Open Questions

- **Periode cooldown sesungguhnya (business days)**: Hasil rename; bisnis perlu memutuskan apakah 3 hari kalender cukup atau perlu 3 hari kerja lengkap dengan kalender hari libur.
- **Coverage 85% — apakah `PgUserRepository` unit test bisa mock-free?**: Akan menggunakan SQLite in-memory (`sqlx::Sqlite`) untuk unit test repository — strategy akan didetailkan di tasks.
