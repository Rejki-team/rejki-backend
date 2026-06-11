## Why

User-service saat ini hanya menyimpan profil minimal (`username`, `full_name`, `avatar`, `bio`, `phone`) dan belum mendukung kelengkapan data diri (KYC) yang diwajibkan onboarding Phase 2. Sesuai user story US-04/US-08/US-09/US-10/US-11/US-12, pengguna wajib melengkapi data diri (termasuk NIK, alamat berjenjang, foto KTP, dan swafoto dengan KTP), data tersebut diverifikasi manual oleh admin, dan pengguna mendapat update progres serta pemberitahuan hasil. Domain ini bergantung pada auth-service (status akun) dan region-service (validasi wilayah) yang dibangun di proposal terpisah.

## What Changes

- **Profil diperluas & dapat diperbarui** (US-08/US-10): lihat profil sendiri lengkap dengan status KYC; perbarui field profil non-KYC; NIK ditampilkan ter-mask.
- **Foto profil** (US-09): unggah/ganti avatar via mekanisme upload aman (presigned URL).
- **Data diri KYC** (US-04): nama lengkap, NIK, tingkat pendidikan, jenis kelamin, tanggal lahir, alamat domisili, negara (default `ID`), provinsi, kabupaten/kota, kecamatan, kelurahan. NIK dienkripsi at-rest; rantai wilayah divalidasi via `RegionClient`.
- **Dokumen KYC** (US-04): unggah foto KTP & swafoto-dengan-KTP via presigned URL ke object storage (MinIO), dengan verifikasi mime/magic bytes, nama objek dibuat server, dan akses non-publik.
- **Submission & alur verifikasi manual admin** (US-04): pengiriman data diri lengkap memicu transisi akun `profile_incomplete → pending_kyc` (via `AuthClient`); admin menyetujui/menolak; approve → `active`, reject → `rejected` (boleh ajukan ulang setelah cooldown 3 hari kerja, K16).
- **NIK tidak dapat diubah** (K5) setelah dikirim; perubahan hanya lewat proses admin khusus.
- **Notifikasi status verifikasi** (US-11/US-12): setiap perubahan status memicu notifikasi push + in-app; email hanya pada saat pengiriman (awal) dan hasil akhir (approved/rejected) — K10/K12 — melalui notification-service.
- **Retensi & penghapusan** (K11): dokumen KYC disimpan selama akun aktif (sah per UU PDP Pasal 7) dan dimusnahkan setelah akun ditutup + periode dispute.

## Capabilities

### New Capabilities
- `user-profile`: lihat & perbarui profil (field non-KYC), tampilan NIK ter-mask, sertakan status KYC.
- `profile-avatar`: unggah/ganti foto profil via presigned URL dengan validasi mime/ukuran.
- `kyc-personal-data`: penyimpanan data diri KYC (termasuk NIK terenkripsi), validasi rantai wilayah, dan aturan NIK immutable.
- `kyc-documents`: unggah & simpan dokumen KTP/swafoto via presigned URL ke object storage, non-publik, dengan verifikasi berkas.
- `kyc-verification-workflow`: submission, transisi status via AuthClient, review admin (approve/reject), cooldown re-submit.
- `kyc-status-notifications`: pemberitahuan progres & hasil verifikasi via notification-service (push/in-app/email sesuai aturan kanal).

### Modified Capabilities
<!-- Tidak ada spec ter-arsip di openspec/specs/. Perluasan profil didefinisikan sebagai kapabilitas baru meski memperluas kode user-service yang ada. -->

## Impact

- **Kode**: `rust-services/user-service/` (domain entity profil + KYC, application service & DTO, infrastructure repository, interface handlers & router), `rust-services/user-service-client/` (perluasan bila diperlukan), `rust-services/rejki-app/` (wiring dependency `AuthClient`, `RegionClient` & `StorageClient` ke user-service).
- **Basis data** (schema `user_svc`): perluasan `profiles` dengan field data diri + `nik_encrypted`/`nik_last4`; tabel `kyc_submission` (status, object key dokumen, reviewer, alasan, waktu). Tanpa FK lintas-schema; kode wilayah disimpan sebagai nilai.
- **API** (`/api/v1/users/*`): perluasan `GET /me`, `PATCH /me`; endpoint baru untuk kirim data KYC, minta presigned URL dokumen & avatar; endpoint admin approve/reject (kelak diproteksi RBAC).
- **Dependensi lintas-domain (in-process)**: `AuthClient` (set/get status akun — dari proposal auth), `RegionClient.validate_chain` (dari proposal region), notification-service (kirim notifikasi), object storage MinIO (presigned URL).
- **Keamanan & PDP**: enkripsi NIK AES-256-GCM (envelope encryption, K14), dokumen non-publik, retensi sesuai UU PDP (K11), IDOR→404 & ownership di query (Authorization Pattern).

## Non-Goals

- Status akun & state machine (dimiliki auth-service; user-service hanya memicu transisi via `AuthClient`).
- Data wilayah & endpoint cascading (dimiliki region-service; user-service hanya memanggil `RegionClient`).
- RBAC admin penuh (PRD Web Dashboard); proposal ini hanya menyediakan endpoint review yang kelak diproteksi RBAC.
- eKYC otomatis pihak ketiga (OCR/liveness/Dukcapil) — tahap awal memakai verifikasi manual admin.
- Internal pengiriman FCM/Bun & verifikasi telepon via SMS.
- Penyelarasan `lokasi` service iklan ke wilayah berstruktur (ditunda).
