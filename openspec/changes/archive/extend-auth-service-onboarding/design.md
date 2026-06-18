## Context

`auth-service` saat ini mendukung register(email, password), verify/resend OTP, login (JWT RS256), refresh (rotation), dan logout. Tabel: `auth.users` (id, email, password_hash, is_verified, created_at, updated_at), `auth.refresh_tokens`, `auth.otp_verifications` (user_id, otp_hash, purpose, expires_at; UNIQUE(user_id, purpose)). Trait `AuthClient { validate_token }` di `auth-service-client`, diimplementasikan `AuthInProcessClient` dan di-wire di `rejki-app`. Seluruh service dikompilasi menjadi satu binary (modular monolith).

Phase 2 menuntut onboarding berbasis status: pengguna baru tidak boleh memakai fitur sampai data diri lengkap & disetujui. Proposal ini menambah fondasi status akun, pemulihan/penggantian password, dan penangguhan — tanpa membongkar alur yang sudah berjalan. Keputusan desain mengacu pada brainstorming K1–K16 (`docs/brainstorm/user-service-phase2.html`) dan standar Phase 1.5.

## Goals / Non-Goals

**Goals:**
- Memperluas auth-service sebagai pemilik tunggal status akun dengan state machine eksplisit.
- Menegakkan gating fitur berbasis status secara cepat (JWT) sekaligus segera-akurat untuk peristiwa sensitif (Redis).
- Menyediakan `AuthClient` yang diperluas agar user-service/admin dapat membaca & mengubah status secara in-process.
- Menambah lupa/ubah password dengan OTP, dan penangguhan akun, mengikuti standar API/keamanan yang ada.
- Memformalkan OTP berbasis `purpose` tanpa menambah tabel baru.

**Non-Goals:**
- Data KYC/profil, data wilayah, RBAC admin penuh, verifikasi SMS, dan internal FCM/Bun (proposal/sistem lain).

## Decisions

### D1 — Status akun sebagai kolom enum-string pada `auth.users`, bukan tabel terpisah
Menambah kolom `status TEXT NOT NULL DEFAULT 'pending_verification'` dengan nilai dari state machine. Alternatif (tabel status terpisah) ditolak karena status adalah atribut 1:1 milik user dan sering dibaca saat auth — kolom lebih sederhana & cepat. `is_verified` lama dipertahankan sementara untuk kompatibilitas, namun sumber kebenaran gating berpindah ke `status`. Transisi divalidasi di application layer (fungsi transisi state machine) agar transisi tak sah ditolak konsisten.

### D2 — Gating hybrid: klaim JWT + pemeriksaan kesegaran Redis (K1)
Status disertakan dalam klaim JWT untuk jalur cepat. Untuk peristiwa sensitif (suspend, perubahan status), ditulis penanda ke Redis (mis. daftar `account_status_changed:{user_id}` / revocation) yang dicek middleware. Alternatif "selalu query DB tiap request" ditolak karena overhead; alternatif "hanya JWT" ditolak karena token basi membuat suspend tertunda hingga 15 menit. Redis sudah tersedia di stack, jadi tidak ada dependensi baru.

**Konsekuensi pada kontrak (T2, diputuskan):** `status` ditambahkan ke `JwtClaims` (internal, di `jwt.rs`) **dan** ke `AuthClaims` publik (`auth-service-client`). Karena `AuthClaims` dipakai oleh handler di seluruh service (chat, notif, 4 iklan), penambahan field ini bersifat lintas-service — namun aman: field baru tidak merusak handler yang hanya membaca `user_id`/`email`. Token yang diterbitkan sebelum migrasi tidak memuat `status`; middleware memperlakukan ketiadaan `status` sebagai sinyal untuk fallback cek kesegaran (Redis/`get_account_status`) demi keamanan.

### D3 — Propagasi status via `AuthClient` in-process, bukan event Redis (K2)
`AuthClient` diperluas: `get_account_status(user_id)` dan `set_account_status(user_id, status)`. Karena auth & user satu binary, ini pemanggilan fungsi langsung — sinkron & konsisten. Event Redis Streams ditolak untuk transisi status (over-engineering untuk in-process; eventual consistency tak diperlukan). Arah ketergantungan satu arah: user/admin → auth. Validasi transisi tetap di auth (pemanggil tak bisa memaksa transisi tak sah).

### D4 — OTP: reuse `otp_verifications` dengan `purpose` (K15)
Purpose baru `reset_password` dan `change_password` memakai tabel & UNIQUE(user_id, purpose) yang ada; pembangkitan baru menggantikan OTP aktif. Tidak ada tabel/mekanisme baru. Tambahkan kolom percobaan (mis. `attempts`) bila belum ada, untuk batas percobaan; rate limit memanfaatkan Redis.

### D5 — Penangguhan: tabel `auth.account_suspension` untuk riwayat + status terderivasi
Tabel `account_suspension(id, user_id, is_permanent, reason, expires_at, created_by, created_at)` menyimpan riwayat & audit. Status user (`suspended_temp`/`suspended_permanent`) diset saat suspend; pengembalian otomatis ke `active` untuk suspend sementara dievaluasi saat login/validasi (lazy: jika `expires_at` lewat, perlakukan active dan/atau perbaiki status). Alternatif "hanya kolom di users" ditolak karena menghilangkan riwayat & audit.

### D6 — Enkripsi `phone` at-rest dengan envelope encryption (K14)
`phone` disimpan terenkripsi AES-256-GCM; KEK di environment secret (sesuai Config Standard fail-fast), DEK dienkripsi oleh KEK. Tanpa KMS berbayar. Email tetap sebagaimana kebutuhan lookup (di luar scope perubahan ini kecuali diperlukan).

### D7 — Kepatuhan standar API Phase 1.5
Semua endpoint memakai envelope `ApiResponse`, error RFC 9457-inspired (`code/message/fields`), prefix `/api/v1/`, status matrix (`422 VALIDATION`, `403 ACCOUNT_NOT_ACTIVE`, `401` untuk OTP/kredensial), propagasi `request_id`. Endpoint baru: `POST /auth/forgot-password`, `POST /auth/reset-password`, `POST /auth/change-password`; body `register` diperluas; kapabilitas suspend disediakan sebagai operasi internal/endpoint yang kelak diproteksi RBAC.

### D8 — Pemasangan middleware auth + gating ke router fitur (T1, diputuskan masuk scope)
Temuan review: `require_auth` (`common/auth-middleware`) **belum terpasang** pada router service fitur — handler iklan menerima `Extension<AuthClaims>` yang tidak pernah di-inject. Karena itu, scope proposal ini mencakup: (a) pemasangan `require_auth` pada router yang membutuhkan identitas, dan (b) penambahan lapisan gating status setelah autentikasi. Gating diimplementasikan sebagai middleware terpisah (mis. `require_active_account`) yang membaca `status` dari `AuthClaims` (jalur cepat) dan melakukan cek kesegaran (D2) untuk peristiwa sensitif, lalu menolak non-`active` dengan `403 ACCOUNT_NOT_ACTIVE`. Endpoint publik (mis. lihat iklan tanpa login) tidak dikenai gating. Pemasangan dilakukan di Composition Root (`rejki-app`) agar wiring terpusat.

### D9 — Migrasi penuh ke `status`, pensiunkan `is_verified` (T4, diputuskan)
`status` menjadi satu-satunya sumber kebenaran kelayakan akun. `login()` yang saat ini memeriksa `is_verified` diubah memeriksa `status` (mis. menolak `pending_verification` dan status suspended). Kolom `is_verified` di-drop setelah backfill `status` (lihat tugas migrasi). Tidak ada dua sumber kebenaran. Verifikasi OTP `register` mengubah `status` `pending_verification → profile_incomplete` (menggantikan efek `mark_verified`).

### D10 — Tambah varian `OtpPurpose::ChangePassword` (T3)
Enum domain `OtpPurpose` saat ini `{ Register, ResetPassword }`. Tambahkan varian `ChangePassword` (string `change_password`) agar OTP ubah-password memakai mekanisme & tabel yang sama (K15). `ResetPassword` sudah ada sehingga lupa-password tidak butuh varian baru.

### D11 — `email` tetap plaintext (T5, dikonfirmasi)
`email` sengaja tidak dienkripsi karena dipakai sebagai kunci lookup login/registrasi & anti-enumeration; hanya `phone` (dan data sensitif lain di user-service seperti NIK) yang dienkripsi at-rest. Keputusan ini eksplisit, bukan kelalaian.

## Risks / Trade-offs

- **Migrasi `status` untuk data lama** → backfill: akun lama `is_verified=true` dipetakan ke status yang sesuai (mis. `profile_incomplete` atau `active` sesuai kebijakan migrasi); akun belum terverifikasi → `pending_verification`. Tetapkan pemetaan eksplisit di migrasi.
- **Konsistensi JWT vs Redis** → ada jendela sangat singkat antara perubahan status dan propagasi penanda; mitigasi: tulis penanda Redis dalam operasi yang sama dengan perubahan status, dan middleware mengutamakan penanda.
- **Kompleksitas state machine** → mitigasi: satu fungsi transisi terpusat + tabel transisi sah; transisi tak sah ditolak seragam.
- **Perluasan `AuthClient` = perubahan kontrak** (BREAKING) → mitigasi: tambah method dengan implementasi in-process; konsumen saat ini hanya `validate_token`, sehingga penambahan tidak merusak pemanggil lama.
- **Ketergantungan pengiriman OTP ke notification-service** → bila notification-service belum siap, sediakan jalur fallback yang aman (gagal tertutup tanpa membocorkan OTP), tetapi alur tetap menyatakan delegasi pengiriman.

## Migration Plan

1. Migrasi DB: tambah kolom `phone`, `status`, `tos_accepted_at`, `tos_version` pada `auth.users`; buat `auth.account_suspension`; pastikan `otp_verifications` mendukung purpose baru (+ kolom `attempts` bila perlu). Backfill `status` dari `is_verified`.
2. Perluas domain/application/infrastructure auth-service (entity, transisi, repo, service).
3. Perluas `auth-service-client` (`AuthClient` + tipe `AccountStatus`), perbarui `AuthInProcessClient`.
4. Tambah endpoint password recovery & kapabilitas suspend; perluas body register + validasi password.
5. Tambah middleware gating (status dari JWT + cek Redis) dan terapkan pada router fitur di `rejki-app`.
6. Integrasikan pengiriman OTP via notification-service (menggantikan TODO log).
7. Rollback: migrasi turun (down) tersedia; kolom baru nullable/aman; `is_verified` dipertahankan sebagai jaring pengaman selama transisi.

## Resolved Questions

Diputuskan oleh pemilik produk (2026-06-11):

- **RQ1 — Backfill `status` akun lama (`is_verified=true`).** Dipetakan ke **`profile_incomplete`**, bukan `active`, agar seluruh pengguna mengikuti alur KYC baru (wajib lengkapi data diri sebelum memakai fitur). Akun belum terverifikasi tetap `pending_verification`. Ini menjadi acuan tugas migrasi 1.2.
- **RQ2 — Nilai TTL OTP, batas percobaan, dan rate limit.** Parameter implementasi; **default konkret diusulkan di `tasks.md`** (lihat grup 5) untuk ditinjau saat implementasi, bukan diputuskan di sini.
- **RQ3 — Bentuk penyimpanan penanda kesegaran Redis (key & TTL).** Detail (skema key & TTL) **diserahkan ke tahap implementasi** mengikuti pola revocation/refresh-status yang dipilih saat itu; spec hanya mewajibkan perilaku kesegaran (D2 / spec `feature-gating`), bukan format penyimpanan.

## Asumsi Implementasi (Perlu Ditinjau)

Lihat [brainstorm § Asumsi](../../../docs/brainstorm/user-service-phase2.html#asumsi) untuk tabel lengkap 14 asumsi yang diambil saat implementasi. Ringkasan yang paling berdampak:

- **A4 — Rate-limit OTP fail-open:** Jika Redis tidak tersedia, batas frekuensi dinonaktifkan (hanya batas percobaan verifikasi via kolom `attempts` yang tetap aktif).
- **A6 — Nama env var kunci enkripsi:** `DATA_ENCRYPTION_KEY` (32 byte base64) — perlu diselaraskan dengan Config Standard bila ada konvensi penamaan resmi.
- **A10 — Backend kripto JWT:** `jsonwebtoken` beralih dari default (`aws-lc-rs`) ke `rust_crypto` untuk menghindari panic CryptoProvider saat sign — perlu ditinjau implikasi performa/audit keamanan.
- **A12 — Binary standalone di-`panic`:** 7 binary standalone (chat, iklan×4, user, notif) ditandai `panic!("... belum didukung — jalankan via rejki-app")`. Auth tetap memiliki binary standalone fungsional.
- **A13 — Perubahan test W8:** Route chat `/rooms` → `/conversations/.../messages` (route lama tidak ada di kode); assertion `data["email"]` → `data["username"]` (profil `user_svc` tidak menyimpan email).
