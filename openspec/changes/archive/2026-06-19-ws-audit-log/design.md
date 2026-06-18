## Context

`auth-service` saat ini mencatat event autentikasi hanya lewat `tracing::info!`/`warn!` (log aplikasi terstruktur). Tidak ada tabel audit immutable yang bisa dipakai untuk investigasi insiden keamanan (brute-force, akun diretas, penyalahgunaan admin). Phase 3 hardening (A3) mensyaratkan tabel `auth.audit_log` sebagai fondasi compliance & forensik.

Tabel `auth.audit_log` sudah didesain di `docs/security-baseline.html` §Audit Log — tinggal diimplementasi.

Service yang disentuh: **hanya auth-service**. Tidak ada endpoint publik baru.

## Goals / Non-Goals

**Goals:**
- Tabel `auth.audit_log` append-only (`REVOKE UPDATE, DELETE`) dengan migration
- `AuditLogRepository` trait (domain) + `PgAuditLogRepository` impl (infrastructure)
- Inject `Arc<dyn AuditLogRepository>` ke `AuthService` dan panggil `log()` di 10+ event auth
- `AuditContext` (ip_address, user_agent, request_id) diekstrak dari header request oleh extractor axum, diteruskan ke service
- Fail-open: kegagalan insert audit log tidak menggagalkan operasi utama
- Unit test AuditLogRepository trait + integration test INSERT + REVOKE

**Non-Goals:**
- Endpoint query/read audit log (itu bagian dari admin dashboard nanti)
- Retensi/archiving policy (nanti via cron job)
- Notifikasi real-time dari audit event
- Audit log untuk service selain auth (chat, iklan, dsb.)

## Decisions

### D1: AuditContext extractor dari header request (bukan ConnectInfo)

**Pilihan**: Ekstrak `ip_address`, `user_agent`, `request_id` dari header HTTP (`x-forwarded-for`, `user-agent`, `x-request-id`) via `FromRequestParts` implementor.

**Alternatif**: `ConnectInfo<SocketAddr>` — butuh layer khusus (`into_make_service_with_connect_info`), tidak available di test, dan tidak memberi user_agent/request_id.

**Rasional**: Header-based lebih portable, mudah di-test (cukup set header di request), dan tidak perlu ubah layer rejki-app. `x-forwarded-for` sudah diset oleh nginx/Cloudflare Tunnel di production.

### D2: `Arc<dyn AuditLogRepository>` (trait object), bukan generic

**Pilihan**: `AuditLogRepository` di-inject sebagai `Arc<dyn AuditLogRepository>` field di `AuthService`.

**Alternatif**: Generic param `AuthService<R, A: AuditLogRepository>` — akan menambah kompleksitas type parameter yang sudah ada, dan tidak memberi manfaat compile-time monomorphization yang signifikan (method `log()` dipanggil sekali per request, bukan di hot loop).

**Rasional**: Konsisten dengan pola `Arc<dyn RateLimiter>` dan `Arc<dyn NotificationClient>` yang sudah ada di `AuthService`. Builder pattern via `with_audit_log()` menjaga backward-compatibilitas test.

### D3: Fail-open — audit log failure tidak menggagalkan operasi utama

**Pilihan**: Bila `audit_log.log()` gagal (DB down, disk full), `AuthService` log `warn!` dan lanjutkan operasi utama.

**Alternatif**: Fail-closed (gagalkan operasi utama) — ini justru menciptakan DoS vector: jika tabel audit_log penuh/bermasalah, seluruh auth service berhenti.

**Rasional**: Audit log adalah observability/forensik, bukan business-critical path. Auth (login/register/refresh) tidak boleh gagal karena audit log.

### D4: AuditContext di-pass eksplisit dari handler ke service method

**Pilihan**: Handler mengekstrak `AuditContext` via extractor axum, lalu meneruskannya sebagai `&AuditContext` ke setiap method `AuthService` yang butuh audit.

**Alternatif**: Middleware yang menyimpan `AuditContext` di task-local / request extension, lalu service baca implicit. Ini "magic" — sulit di-test, tidak eksplisit di signature.

**Rasional**: Eksplisit di parameter method — jelas di type signature bahwa method tersebut mencatat audit. Test bisa inject `AuditContext` buatan tanpa setup HTTP.

### D5: AuditEvent sebagai enum dengan variant-specific metadata

**Pilihan**: `AuditEvent` enum dengan variant yang membawa metadata (mis. `OtpSent { purpose: String }`, `AccountSuspended { admin_id, reason, permanent }`). Disimpan ke DB sebagai string event + JSONB metadata.

**Alternatif**: Free-form string event + arbitrary metadata map — kurang type-safe, rawan typo.

**Rasional**: Enum menjaga konsistensi nama event. Metadata JSONB fleksibel untuk data tambahan per event.

## Risks / Trade-offs

- **Risk**: Audit log table grow tanpa batas → **Mitigasi**: Di luar scope W3B-04; retensi bisa ditambahkan via cron job (`DELETE WHERE created_at < now() - INTERVAL '90 days'`) di task terpisah.
- **Risk**: INSERT ke audit_log menambah latency tiap auth request (~1-2ms) → **Mitigasi**: Acceptable trade-off; tabel append-only (tanpa index selain PK + user_id), INSERT sangat cepat.
- **Risk**: `x-forwarded-for` bisa di-spoof bila nginx tidak dikonfigurasi benar → **Mitigasi**: Di luar scope; konfigurasi nginx/Cloudflare sudah menangani ini (`real_ip_header`).

## Migration Plan

1. Jalankan migration `20260618000001_create_audit_log.up.sql` via `sqlx migrate run`
2. Deploy kode baru (fail-open: audit_log None → skip)
3. Tidak perlu rollback — migration hanya CREATE TABLE, kode baru backward-compatible
4. `cargo sqlx prepare --workspace` setelah migration untuk update query cache
