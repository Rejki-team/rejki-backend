## 1. Refactor suspend single → reusable (spec: bulk-user-suspension)

- [ ] 1.1 Ekstrak logika suspend per-user dari handler single menjadi method service privat `suspend_one(user_id, params)` (tanpa mengubah endpoint single)
- [ ] 1.2 Pastikan endpoint single `POST /auth/admin/users/{id}/suspend` tetap berfungsi (regресi)

## 2. Endpoint bulk (spec: bulk-user-suspension)

- [ ] 2.1 DTO `BulkSuspendInput { user_ids: Vec<Uuid>, permanent, reason, evidence_object_key, expires_at }` + validasi (reason wajib; expires_at wajib bila sementara; batas maks user_ids mis. 100)
- [ ] 2.2 DTO `BulkSuspendResponse { results: Vec<{ user_id, success, error? }> }`
- [ ] 2.3 Service `suspend_accounts_bulk` — validasi awal (fail-fast) → iterasi `suspend_one` per user → kumpulkan hasil per-item
- [ ] 2.4 Handler `suspend_accounts_bulk` → `POST /api/v1/auth/admin/users/suspend` (200 + array), proteksi `require_admin`
- [ ] 2.5 Notifikasi email + in-app per user sukses (reuse NotificationClient; best-effort + logged)

## 3. Purge lintas-service saat permanen (spec: permanent-suspend-purge)

- [ ] 3.1 Tambah method trait `UserClient::purge_kyc_documents(user_id) -> Result<(), UserClientError>` di `user-service-client`
- [ ] 3.2 Implementasi in-process di `user-service` (resolve profile dari user_id/auth_id → `purge_documents`)
- [ ] 3.3 Wire `UserClient` ke `auth-service` (state) via `rejki-app` composition root
- [ ] 3.4 Saat `permanent=true` & item sukses: panggil `purge_kyc_documents` (best-effort + logged)
- [ ] 3.5 Pastikan suspend sementara TIDAK memicu purge

## 4. OpenAPI & dokumentasi

- [ ] 4.1 Anotasi `utoipa::path` untuk endpoint bulk di `rejki-app/openapi.rs`
- [ ] 4.2 Update `add-user-service-kyc/tasks.md` `[6.4 trigger]` → resolved untuk jalur suspend permanen

## 5. Pengujian

- [ ] 5.1 Test bulk: semua sukses; sebagian gagal (user invalid) → hasil per-item benar; batch tidak batal total
- [ ] 5.2 Test validasi fail-fast: reason kosong / expires_at kosong saat sementara
- [ ] 5.3 Test batas maksimum user_ids
- [ ] 5.4 Test purge terpicu saat permanen; tidak terpicu saat sementara; kegagalan purge tidak membatalkan suspend
- [ ] 5.5 Test notifikasi email + in-app terkirim per user sukses
- [ ] 5.6 Test non-admin ditolak

## 6. Verifikasi

- [ ] 6.1 `cargo build` workspace hijau (online + offline `.sqlx`)
- [ ] 6.2 `cargo fmt` + `cargo clippy` bersih
- [ ] 6.3 Cek silang: requirement spec ↔ test ↔ FR-ADM-USR-07 di PRD
