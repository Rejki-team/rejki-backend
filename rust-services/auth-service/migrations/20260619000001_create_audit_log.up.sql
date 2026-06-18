-- Tabel audit_log — append-only immutable record setiap auth event.
-- Digunakan untuk investigasi insiden keamanan dan compliance.
-- Phase 3 hardening (A3) — W3B-04.

CREATE TABLE IF NOT EXISTS auth.audit_log (
    id          UUID        NOT NULL DEFAULT gen_random_uuid() PRIMARY KEY,
    user_id     UUID,                           -- NULL untuk failed login (user tidak dikenal)
    event       TEXT        NOT NULL,           -- 'login_success', 'login_failed', 'logout', dsb.
    ip_address  INET,                           -- IP pengguna (dari x-forwarded-for)
    user_agent  TEXT,                           -- User-Agent header
    request_id  TEXT,                           -- x-request-id (UUID v7)
    metadata    JSONB,                          -- data tambahan per event (tidak menyimpan password/token)
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Index untuk query berdasarkan user_id (investigasi per-akun).
CREATE INDEX IF NOT EXISTS idx_audit_log_user_id ON auth.audit_log (user_id);

-- Index untuk query berdasarkan waktu (investigasi temporal).
CREATE INDEX IF NOT EXISTS idx_audit_log_created_at ON auth.audit_log (created_at);

-- Append-only: UPDATE dan DELETE dari tabel ini hanya boleh dilakukan oleh superuser/DBA.
-- Aplikasi (PgAuditLogRepository) hanya melakukan INSERT — enforce di application layer.
-- Di production, jalankan secara manual:
--   REVOKE UPDATE, DELETE ON auth.audit_log FROM <app_role>;
