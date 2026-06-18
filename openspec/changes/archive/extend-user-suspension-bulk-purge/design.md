## Context

Suspend pengguna single sudah ada (`POST /auth/admin/users/{id}/suspend` + endpoint evidence) dari
`extend-auth-service-onboarding`, beserta tabel `auth.account_suspension` (id, user_id, is_permanent,
reason, evidence_object_key, expires_at, created_by, created_at). Pemusnahan dokumen KYC tersedia sebagai
`UserService::purge_documents` ([user-service/service.rs:429](../../../rust-services/user-service/src/application/service.rs#L429))
namun belum dipicu otomatis. Komunikasi lintas-service memakai pola **in-process client trait** —
`UserClient` ([user-service-client/src/lib.rs:13](../../../rust-services/user-service-client/src/lib.rs#L13)).

## Goals / Non-Goals

**Goals:** bulk suspend pengguna dengan partial-success; purge dokumen saat permanen; notifikasi email + in-app.

**Non-Goals:** unsuspend massal; purge saat penutupan akun; UI Vue; listing pengguna (Change A).

## Decisions

### D1 — Endpoint bulk baru, single dipertahankan
`POST /api/v1/auth/admin/users/suspend` dengan body `BulkSuspendInput { user_ids: Vec<Uuid>, permanent,
reason, evidence_object_key, expires_at }`. Endpoint single `/{id}/suspend` tetap ada agar tidak breaking.
Pola ini meniru suspend iklan yang sudah bulk (`iklan_ids: Vec<Uuid>`,
[iklan-pekerja/dto.rs:77](../../../rust-services/iklan-pekerja-service/src/application/dto.rs#L77)).

### D2 — Respons partial-success (200 + array per-item), konsisten internal
`BulkSuspendResponse { results: Vec<{ user_id, success, error? }> }`, mengembalikan `200 OK`.
Dipilih demi **konsistensi dengan `SuspendResponse` iklan yang sudah ada**
([iklan-pekerja/dto.rs:87-97](../../../rust-services/iklan-pekerja-service/src/application/dto.rs#L87)).
Alternatif standar industri **HTTP 207 Multi-Status** dipertimbangkan
([OneUptime](https://oneuptime.com/blog/post/2026-02-02-rest-bulk-api-partial-success/view),
[Apidog 207](https://apidog.com/blog/status-code-207-multi-status/)) — tidak dipakai agar klien tidak perlu
menangani kelas status non-standar; trade-off ini didokumentasikan. Validasi awal (reason wajib;
`expires_at` wajib bila sementara) **fail-fast sebelum** memproses item.

### D3 — Suspend per-user reuse logika single (no God function)
Service melakukan iterasi `user_ids`, memanggil fungsi suspend-per-user yang sudah ada (refactor logika
single menjadi method privat `suspend_one`). Tiap item ditangani independen; galat satu item → `success=false`
+ `error`, tidak menghentikan batch. Menghindari duplikasi & God function.

### D4 — Purge lintas-service saat permanen via `UserClient`
Tambah method trait `UserClient::purge_kyc_documents(user_id) -> Result<(), UserClientError>`. Implementasi
in-process di `user-service` memanggil `purge_documents(profile_id)` (resolve profile dari user_id/auth_id).
Saat `permanent=true` dan suspend item sukses, auth-service memanggil purge **best-effort + logged**
(kegagalan purge tidak membatalkan suspend yang sudah tercatat). Idempoten (purge aman dipanggil ulang).

### D5 — Notifikasi email + in-app per pengguna terdampak
Reuse `NotificationClient` (`send` in-app + `send_email`). Dikirim untuk tiap user sukses. Best-effort + logged
(pola yang sudah dipakai KYC, [user-service/service.rs:461-485](../../../rust-services/user-service/src/application/service.rs#L461)).

### D6 — Bukti wajib mengikuti alur evidence yang sudah ada
`evidence_object_key` diperoleh klien lebih dulu via endpoint evidence presigned yang sudah ada
(`POST /auth/admin/users/{id}/suspend/evidence` atau varian batch bila perlu). Validasi ≤5MB & 1 file
ditegakkan saat penerbitan presigned (sudah ada di `SuspendEvidenceRequest`).

## Risks / Trade-offs

- **Partial success vs transaksional**: batch non-transaksional — sebagian pengguna bisa ter-suspend, sebagian gagal. Diterima; hasil per-item memberi klien info untuk retry selektif ([Zalando #127](https://github.com/zalando/restful-api-guidelines/issues/127)).
- **Purge best-effort**: bila user-service/storage down, dokumen mungkin tertinggal saat permanen; dicatat untuk job pembersih. Suspend tetap berlaku.
- **Ukuran batch**: perlu batas maksimum `user_ids` (mis. 100) agar request tidak terlalu besar; didokumentasikan & divalidasi.

## Migration Plan

1. Refactor logika suspend single → `suspend_one` (private), tanpa mengubah endpoint single.
2. Tambah DTO bulk + handler + route `POST /auth/admin/users/suspend`.
3. Tambah `UserClient::purge_kyc_documents` + implementasi in-process; wire `UserClient` ke auth-service di `rejki-app`.
4. Picu purge saat `permanent=true`; kirim notifikasi email + in-app per user sukses.
5. Update OpenAPI + `add-user-service-kyc/tasks.md` `[6.4 trigger]` → resolved untuk jalur suspend permanen.
6. Uji: bulk sebagian gagal; purge terpicu saat permanen; notifikasi terkirim; batas ukuran batch.

## Open Questions

- **Batas maksimum `user_ids`** per request: default 100 (selaras praktik dokumentasi limit batch). Konfirmasi pemilik bila beda.
- **Evidence per-batch vs per-user**: satu bukti untuk seluruh batch atau per-user? Default: satu `evidence_object_key` untuk batch (alasan suspend kolektif). Konfirmasi bila perlu per-user.
