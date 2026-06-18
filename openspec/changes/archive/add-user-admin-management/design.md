## Context

`user-service` sudah memiliki domain KYC penuh (profil, submission, dokumen, audit `document_access_log`,
NIK terenkripsi + `nik_last4`) dari `add-user-service-kyc`. Yang hilang adalah **antarmuka admin**:
listing pengajuan, akses dokumen teraudit, dan auto-purge saat reject. RBAC (`require_admin`) sudah tersedia
dari `add-admin-rbac`. Change ini bersifat **aditif** pada service yang sudah ada — tanpa crate/schema baru.

Acuan kontrak: envelope `ApiResponse<T>` = `{ success, data, meta?, request_id }`
([common/errors/src/lib.rs:95](../../../rust-services/common/errors/src/lib.rs#L95)); paginasi `PaginatedMeta`
= `{ page, per_page, total, total_pages }` ([lib.rs:137](../../../rust-services/common/errors/src/lib.rs#L137));
query admin mengikuti pola existing `{ q, status, sort_by, sort_dir, limit, offset }`.

## Goals / Non-Goals

**Goals:** listing + detail KYC admin; akses dokumen admin teraudit; auto-purge saat reject; CSV; penguncian pasca-verifikasi.

**Non-Goals:** bulk suspend (Change B); purge saat suspend permanen (Change B); purge saat penutupan akun; UI Vue.

## Decisions

### D1 — Path admin di bawah namespace service: `/users/admin/kyc*`
Mengikuti pola yang sudah dipakai service lain (`{service}/admin/...`, mis. `/pekerja/admin/`, `/pelatihan/admin/pelatihan`).
Router admin di-`nest("/admin", ...)` di dalam `user_service::router`, dilapisi `require_admin` + `require_auth`
(default-deny). Konsisten dengan [user-service/interface/mod.rs:74-84](../../../rust-services/user-service/src/interface/mod.rs#L74)
yang sudah meletakkan `review_kyc` di `/admin/kyc/{id}/review`.

### D2 — Listing tidak membawa NIK; NIK penuh hanya via click-to-view teraudit
Response listing memuat `nik_masked` (`xxx...1234`) saja, sejalan praktik **PII masking + audit akses**
([Zuplo](https://zuplo.com/learning-center/protect-sensitive-data-in-api-logs)). NIK penuh **tidak** pernah
dikembalikan secara massal. Bila admin perlu melihat NIK penuh, gunakan mekanisme click-to-view yang
mencatat audit (selaras dokumen) — pada iterasi ini, fokus dokumen KTP/Selfie; NIK penuh dapat ditambahkan
sebagai endpoint teraudit terpisah bila pemilik produk meminta (open question).

### D3 — Akses dokumen admin mencatat audit `read_issued` dengan aktor admin
`GET /users/admin/kyc/{id}/documents/{kind}` memanggil ulang pola `get_document_url` yang sudah ada
([service.rs:380-407](../../../rust-services/user-service/src/application/service.rs#L380)) tetapi:
- resolusi `submission` berdasarkan **submission id / profile target** (bukan `claims.user_id`),
- `log_document_access(admin_id, key, ReadIssued, request_id)` mencatat **admin** sebagai aktor (akuntabilitas),
- presigned read URL berumur pendek (sesuai konfigurasi storage existing).
Memenuhi FR-ADM-USR-02 (click-to-view teraudit) & NFR audit trail ([OWASP Authz](https://cheatsheetseries.owasp.org/cheatsheets/Authorization_Cheat_Sheet.html)).

### D4 — Auto-purge saat reject: panggil `purge_documents` di dalam `review_kyc`
Pada cabang `approved=false` di [service.rs:274-288](../../../rust-services/user-service/src/application/service.rs#L274),
setelah status di-set `Rejected` dan notifikasi terkirim, panggil `self.purge_documents(profile.id)`.
Purge bersifat **best-effort + logged** (pola yang sudah ada: `tracing::warn!` saat gagal hapus per-key,
[service.rs:438-446](../../../rust-services/user-service/src/application/service.rs#L438)) agar kegagalan storage
tidak membatalkan keputusan review yang sudah tercatat. Idempoten: `purge_documents` aman dipanggil ulang
([service.rs:429-433](../../../rust-services/user-service/src/application/service.rs#L429), early-return bila tak ada dokumen).

### D5 — Penguncian pasca-verifikasi (idempotensi review)
`review_kyc` menolak (`409`/validation) bila submission sudah berstatus terminal (`approved|rejected`).
Mencegah pembukaan ulang dokumen yang telah di-purge & menjaga integritas audit (FR-ADM-USR-06).

### D6 — Resolve nama wilayah untuk kolom Alamat/Wilayah
Kolom "Kombinasi wilayah Kelurahan→Negara" membutuhkan resolusi `province_id/regency_id/district_id/village_id`
(disimpan saat submit KYC) menjadi nama. Gunakan `RegionClient` (sudah di-inject ke `UserService`,
[mod.rs:23-41](../../../rust-services/user-service/src/interface/mod.rs#L23)). Untuk listing besar, hindari N+1:
resolve batch atau sertakan id + biarkan UI memanggil cascading region API (open question — default: sertakan
id wilayah di response, biarkan UI resolve via `region-service` yang sudah ada).

## Risks / Trade-offs

- **N+1 region lookup** pada listing → mitigasi D6 (sertakan id, UI resolve; atau batch resolve di service).
- **Purge best-effort** → bila storage down saat reject, dokumen mungkin tertinggal; dicatat di log untuk job pembersih lanjutan. Keputusan review tetap konsisten.
- **Audit volume** → setiap click-to-view menambah baris `document_access_log`; ini memang diinginkan (akuntabilitas).

## Migration Plan

1. Tambah handler + service method + query repo (listing, detail, document, csv).
2. Wire `purge_documents` ke cabang reject `review_kyc` + guard idempotensi.
3. Tambah index DB pendukung listing bila perlu (migrasi aditif).
4. Update OpenAPI (`rejki-app/openapi.rs`) untuk endpoint baru.
5. Uji integrasi: listing pagination/search/sort; akses dokumen mencatat audit; reject memicu purge; review ulang ditolak.

## Open Questions

- **NIK penuh teraudit**: perlukah endpoint `GET /admin/kyc/{id}/nik` teraudit terpisah, atau cukup `nik_masked`? (default: masked saja iterasi ini).
- **Region resolve**: di service (batch) atau di UI (cascading API)? (default: id di response, UI resolve).
