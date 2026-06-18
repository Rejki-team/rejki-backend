## 1. Refactor suspend single → reusable (spec: bulk-user-suspension)

- [x] 1.1 Ekstrak logika suspend per-user dari handler menjadi method service privat `suspend_one(user_id, params)` (tanpa mengubah endpoint single)
- [x] 1.2 Pastikan endpoint single `POST /auth/admin/users/{id}/suspend` tetap berfungsi (regresi)

## 2. Endpoint bulk (spec: bulk-user-suspension)

- [x] 2.1 DTO `BulkSuspendInput { user_ids: Vec<Uuid>, permanent, reason, evidence_object_key, expires_at }` + validasi (reason wajib; expires_at wajib bila sementara; batas maks user_ids 100)
- [x] 2.2 DTO `BulkSuspendResponse { results: Vec<{ user_id, success, error? }> }`
- [x] 2.3 Service `suspend_accounts_bulk` — validasi awal (fail-fast) → iterasi `suspend_one` per user → kumpulkan hasil per-item
- [x] 2.4 Handler `suspend_accounts_bulk` → `POST /api/v1/auth/admin/users/suspend` (200 + array), proteksi `require_admin` (segmen `/admin` sesuai konvensi codebase)
- [x] 2.5 Notifikasi in-app + email per user sukses (reuse NotificationClient `send` + `send_email`; best-effort + logged) — D5 mewajibkan email + in-app

## 3. Purge lintas-service saat permanen (spec: permanent-suspend-purge)

- [x] 3.1 Tambah method trait `UserClient::purge_kyc_documents(user_id) -> Result<(), UserClientError>` di `user-service-client`
- [x] 3.2 Implementasi in-process di `user-service` (UserInProcessClient → purge_kyc_by_user_id → resolve_profile_id → purge_documents)
- [x] 3.3 Wire `UserClient` ke `auth-service` (AppState) via `rejki-app` composition root
- [x] 3.4 Saat `permanent=true` & item sukses: panggil `purge_kyc_documents` (best-effort + logged)
- [x] 3.5 Pastikan suspend sementara TIDAK memicu purge

## 4. OpenAPI & dokumentasi

- [x] 4.1 Anotasi `utoipa::path` untuk endpoint bulk di `rejki-app/openapi.rs` (`suspend_bulk_doc` + `BulkSuspendDocRequest`/`BulkSuspendDocResponse`); path single dikoreksi ke `/auth/users/...` (sesuai route aktual)
- [x] 4.2 Update `add-user-service-kyc/tasks.md` `[6.4 trigger]` → resolved untuk jalur suspend permanen

## 5. Pengujian

- [x] 5.1 Test bulk: semua sukses; sebagian gagal (user invalid) → hasil per-item benar; batch tidak batal total
- [x] 5.2 Test validasi fail-fast: reason kosong → 422, expires_at kosong saat sementara → 422
- [x] 5.3 Test batas maksimum user_ids (validasi `#[validate(length(min = 1, max = 100))]` di DTO)
- [x] 5.4 Test purge terpicu saat permanen (positif: `..._documents_purged`, verifikasi key dikosongkan) + tidak terpicu saat sementara; harness test kini wire `user_client` (mirror main.rs) agar jalur D4 ter-cover
- [x] 5.5 Test notifikasi in-app terkirim per user sukses (best-effort)
- [x] 5.6 Test non-admin ditolak; anon ditolak
- [x] 5.7 Test single endpoint masih berfungsi (regresi)

## 6. Verifikasi

- [x] 6.1 `cargo build` workspace hijau
- [x] 6.2 `cargo fmt` + `cargo clippy` bersih
- [x] 6.3 Cek silang: requirement spec ↔ test ↔ FR-ADM-USR-07 di PRD
- [x] 6.4 Regression: 35 test existing lulus (13 auth + 13 admin_kyc + 9 barang_gratis)
