## Why

Rejki Web Dashboard ([prd-dashboard.md](../../../docs/prd/prd-dashboard.md) §5.7, FR-ADM-USR-07) menuntut admin dapat **men-suspend pengguna secara sekaligus** ("salah satu, sebagian, atau **sekaligus**") dan mewajibkan **Foto KTP/Selfie terhapus otomatis jika suspend permanen**. Penelusuran kode aktual ([dashboard-gap-analysis.md](../../../docs/dashboard-gap-analysis.md) §2.5) menemukan dua gap:

1. **Suspend pengguna hanya single** — `POST /auth/admin/users/{id}/suspend` menerima satu `id` di path dan `SuspendInput` tanpa array ([auth-service/application/dto.rs:111-122](../../../rust-services/auth-service/src/application/dto.rs#L111)). Bandingkan: suspend **iklan** sudah mendukung bulk via `iklan_ids: Vec<Uuid>`. User Story menuntut bulk untuk pengguna juga.
2. **Auto-purge dokumen saat suspend permanen tidak ada trigger** — tidak ada pemanggilan purge dokumen dari alur suspend; dicatat pending di `add-user-service-kyc/tasks.md` baris 36 (`[6.4 trigger]`). Suspend permanen mewajibkan dokumen KYC dimusnahkan (UU PDP).

Change ini menambah jalur bulk + memicu pemusnahan dokumen lintas-service (auth → user-service) saat permanen, dengan respons partial-success yang konsisten dengan pola `SuspendResponse` iklan yang sudah ada.

## What Changes

- **Bulk suspend pengguna** (FR-ADM-USR-07): endpoint `POST /api/v1/auth/admin/users/suspend` menerima `user_ids: [UUID]`, `permanent: bool`, `reason` (wajib), `evidence_object_key` (bukti ≤5MB, 1 file — alur evidence yang sudah ada), `expires_at` (wajib bila sementara). Respons **partial-success**: array per-item `{ user_id, success, error? }` sehingga sebagian gagal tidak menggagalkan seluruh batch. Endpoint single yang ada tetap dipertahankan (kompatibilitas).
- **Pemusnahan dokumen saat permanen** (FR-ADM-USR-07): saat `permanent=true`, untuk tiap pengguna yang berhasil di-suspend, auth-service memicu pemusnahan dokumen KYC via `UserClient` (in-process). Bersifat best-effort + logged.
- **Notifikasi otomatis** (FR-ADM-USR-07): tiap pengguna terdampak menerima notifikasi **email + in-app** atas suspend yang dilakukan (reuse `NotificationClient`).

## Capabilities

### New Capabilities
- `bulk-user-suspension`: suspend banyak pengguna dalam satu permintaan dengan alasan + bukti wajib dan hasil per-item (partial success).
- `permanent-suspend-purge`: pemusnahan otomatis dokumen KYC pengguna saat suspend permanen, lintas-service.

## Impact

- **Kode**: `rust-services/auth-service/` — handler `suspend_accounts_bulk` + DTO `BulkSuspendInput`/`BulkSuspendResponse`; service iterasi per-user (reuse logika suspend single yang ada). `rust-services/user-service-client/` — tambah method trait `purge_kyc_documents(user_id)`; implementasi in-process di `user-service` (reuse `purge_documents` yang sudah ada). Wiring `UserClient` ke `auth-service` di `rejki-app`.
- **Basis data**: reuse `auth.account_suspension` (sudah ada) — satu baris per pengguna ter-suspend. Tanpa schema baru.
- **API**: `POST /api/v1/auth/admin/users/suspend` (bulk). Endpoint single `POST /auth/admin/users/{id}/suspend` tetap ada.
- **Dependensi**: `add-admin-rbac` (proteksi), `extend-auth-service-onboarding` (suspend + evidence existing), `add-user-service-kyc` (`purge_documents`), `NotificationClient`.
- **Standar**: envelope `ApiResponse`, partial-success array (selaras `SuspendResponse` iklan), propagasi `request_id`, default-deny RBAC.

## Non-Goals

- UI dashboard (Vue) — `add-rejki-web-dashboard`.
- Listing pengguna / akses dokumen admin — `add-user-admin-management` (Change A).
- Purge saat penutupan akun (event lifecycle) — tetap pending untuk jalur non-suspend.
- Unsuspend/restore massal — di luar scope iterasi ini.
