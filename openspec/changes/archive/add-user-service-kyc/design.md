## Context

User-service saat ini menyimpan profil minimal di `user_svc.profiles` (`username`, `full_name`, `avatar`, `bio`, `phone`) dengan handler `get_me`/`get_by_id`/`update_me`. Onboarding Phase 2 (US-04, US-08–US-12) menuntut data diri KYC lengkap + dokumen KTP/swafoto, verifikasi manual admin, dan notifikasi status. Domain ini bergantung pada dua proposal lain: auth-service yang diperluas (kepemilikan status akun + `AuthClient.set/get_account_status`) dan region-service (`RegionClient.validate_chain`). Seluruh service satu binary (in-process). Keputusan acuan: K1, K2, K5, K9, K10, K11, K12, K13, K14, K16 dan US-04/08/09/10/11/12.

## Goals / Non-Goals

**Goals:**
- Memperluas profil & menyediakan data diri KYC, dokumen, alur verifikasi manual, dan notifikasi status.
- Memicu transisi status akun via `AuthClient` (bukan menyimpan status di user-service).
- Memvalidasi wilayah via `RegionClient`, mengenkripsi NIK, dan menjaga dokumen non-publik.

**Non-Goals:**
- Kepemilikan status akun (auth), data wilayah (region), RBAC admin penuh, eKYC otomatis, internal FCM/Bun, verifikasi SMS, penyelarasan `lokasi` iklan.

## Decisions

### D1 — Pisahkan submission KYC dari tabel profil
Profil diperluas dengan field data diri + `nik_encrypted`/`nik_last4`, tetapi siklus verifikasi disimpan di tabel `user_svc.kyc_submission` (status `pending/approved/rejected`, object key dokumen, `reviewed_by`, `review_note`, timestamps). Pemisahan ini menyimpan riwayat pengajuan & audit review, dan memudahkan re-submit. Alternatif "semua flag di profil" ditolak (kehilangan riwayat).

### D2 — Status akun tetap milik auth-service (K2)
User-service TIDAK menyimpan status akun. Saat submission lengkap → panggil `AuthClient.set_account_status(user_id, pending_kyc)`; approve → `active`; reject → `rejected`. Pembacaan status untuk ditampilkan di profil memakai `get_account_status`. Transisi divalidasi di auth (user-service tak bisa memaksa transisi tak sah). Pemanggilan in-process, sinkron.

### D3 — Reviewer "manusia/sistem" agar mudah upgrade ke eKYC (brainstorm K-KYC)
`kyc_submission` memodelkan keputusan review sebagai `approved/rejected + reviewer + alasan`. Saat ini reviewer = admin (manual). Bila kelak memakai eKYC pihak ketiga, reviewer bisa "sistem" tanpa membongkar skema.

### D4 — Validasi wilayah via RegionClient sebelum simpan (K13)
Data wilayah disimpan sebagai kode (province_id/regency_id/district_id/village_id) tanpa FK lintas-schema; konsistensi dijamin dengan memanggil `RegionClient.validate_chain` saat tulis. Negara default `ID`.

### D5 — NIK terenkripsi & immutable (K5, K14)
NIK dienkripsi AES-256-GCM (envelope encryption, KEK dari environment), disertai `nik_last4` untuk tampilan ter-mask. Setelah dikirim, NIK tidak dapat diubah via self-service (`PATCH /me` menolak perubahan NIK). Memakai utilitas enkripsi yang sama dengan auth-service (phone) bila tersedia.

### D6 — Upload via presigned URL ke MinIO, dokumen non-publik (K7)
Alur dua langkah: minta izin (validasi mime/ukuran) → presigned URL + object key dibuat server → klien unggah langsung; commit memverifikasi magic bytes. Object key disimpan (bukan URL publik). Akses baca dokumen hanya pemilik & admin via presigned read berumur pendek. Berlaku untuk dokumen KYC dan avatar (avatar boleh publik/terbatas sesuai kebutuhan; dokumen WAJIB non-publik).

### D7 — Notifikasi via notification-service dengan aturan kanal (K10/K12)
Setiap perubahan status → event ke notification-service (push + in-app). Email hanya pada awal (submission dibuat) & hasil akhir (approved/rejected). Memakai jalur Redis Streams → Bun → FCM yang sudah ada; user-service hanya menerbitkan event/permintaan kirim, tidak mengurus internal FCM.

### D8 — Kepatuhan standar Phase 1.5
Envelope `ApiResponse`, error RFC 9457-inspired, prefix `/api/v1/`, IDOR→404 & ownership di query (Authorization Pattern), snake_case DB, `created_at`/`updated_at`, propagasi `request_id`.

## Risks / Trade-offs

- **Ketergantungan pada 2 proposal lain (auth & region)** → user-service-kyc sebaiknya diimplementasikan SETELAH auth diperluas & region-service tersedia; antarmuka `AuthClient`/`RegionClient` menjadi kontrak integrasi.
- **Konsistensi transisi status lintas domain** → karena in-process & sinkron (D2), transisi dan penyimpanan submission dapat diurutkan dalam satu alur; tangani kegagalan parsial (mis. submission tersimpan tapi set_status gagal) dengan urutan operasi yang aman + retensi idempoten.
- **Kebocoran data sensitif** → NIK terenkripsi + ter-mask; dokumen non-publik; NIK/dokumen tidak pernah di-log; akses via ownership query (404 untuk non-pemilik).
- **Pemusnahan dokumen (retensi K11)** → perlu proses penghapusan saat akun ditutup + dispute; detail trigger penutupan akun bergantung kebijakan akun (lintas auth) — ditandai di Resolved/Open.

## Migration Plan

1. Migrasi `user_svc.profiles`: tambah field data diri + `nik_encrypted`, `nik_last4`, kolom wilayah (kode), `country_code` default `ID`.
2. Buat tabel `user_svc.kyc_submission`.
3. Implementasi enkripsi NIK + util upload presigned (MinIO) + verifikasi magic bytes.
4. Implementasi endpoint profil (perluasan), avatar, kirim data KYC, dokumen, dan review admin.
5. Integrasi `AuthClient` (transisi status), `RegionClient` (validasi wilayah) & `StorageClient` (presigned upload avatar/dokumen) via wiring di `rejki-app`.
6. Integrasi notifikasi status via notification-service (aturan kanal K10/K12).
7. Rollback: migrasi turun; kolom baru nullable; tabel submission dapat di-drop tanpa memengaruhi auth/region.

## Resolved Questions

Diputuskan oleh pemilik produk pada sesi brainstorming (K-series):
- **NIK immutable** (K5), **enkripsi via envelope encryption KEK di env** (K14), **storage MinIO presigned** (K7), **retensi selama akun aktif lalu dimusnahkan setelah penutupan + dispute** (K11), **email hanya awal & hasil akhir** (K12), **cooldown re-submit 3 hari kerja** (K16), **wilayah 4 tingkat + default negara ID** (K13).

Diputuskan 2026-06-13 (jawaban Open Questions pasca-code review):
- **Q1 — Suspend wajib bukti**: Admin yang men-suspend akun (sementara/permanen) WAJIB menyertakan alasan + bukti (gambar/dokumen, maks 5MB per dokumen) demi keamanan audit & sengketa. Kolom `evidence_object_key` ditambah di `auth.account_suspension`; endpoint evidence presigned URL ditambah di auth-service via `StorageClient` kategori `suspension-evidence`. Selaras US-07, PRD Dashboard FR-ADM-USR-03 & Audit Log.
- **Q2 — Audit trail akses dokumen**: Setiap akses dokumen KYC wajib terlacak: kapan diupload (presigned upload diterbitkan), kapan dibuka/dilihat (presigned read diterbitkan), dan kapan di-commit. Dicatat di tabel append-only `user_svc.document_access_log` (actor_id, object_key, action, request_id, occurred_at). Action: `upload_issued`, `commit`, `read_issued`. Pemenuhan audit & sengketa sesuai UU PDP.

## Open Questions

- **Pemicu penutupan akun untuk retensi (K11)**: mekanisme & kepemilikan "akun ditutup" (kemungkinan lintas auth-service) belum didefinisikan; periode dispute pasti (mis. 30 hari) perlu ditetapkan kebijakan. Method `purge_documents(profile_id)` sudah tersedia di user-service; pemanggilnya menunggu event penutupan akun dari auth-service.
- **Aturan akses baca dokumen oleh admin**: endpoint `GET /me/documents/{kind}` saat ini owner-only; admin read ditunda hingga RBAC tersedia (PRD Dashboard).
