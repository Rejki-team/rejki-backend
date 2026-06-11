-- Extend auth-service untuk onboarding Phase 2
-- Ref: openspec/changes/extend-auth-service-onboarding

-- 1.1 Kolom baru pada auth.users
ALTER TABLE auth.users
    ADD COLUMN IF NOT EXISTS phone           TEXT,            -- E.164, terenkripsi at-rest (disimpan sebagai ciphertext base64)
    ADD COLUMN IF NOT EXISTS status          TEXT NOT NULL DEFAULT 'pending_verification',
    ADD COLUMN IF NOT EXISTS tos_accepted_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS tos_version     TEXT;

-- 1.2 Backfill status dari is_verified (RQ1):
--   belum verifikasi  -> pending_verification (sudah default)
--   sudah  verifikasi  -> profile_incomplete (mengikuti alur KYC baru, tidak langsung active)
UPDATE auth.users SET status = 'profile_incomplete' WHERE is_verified = true;

-- 1.3 Drop is_verified setelah backfill (status jadi satu-satunya sumber kebenaran)
ALTER TABLE auth.users DROP COLUMN IF EXISTS is_verified;

-- Batasi nilai status agar konsisten dengan state machine domain
ALTER TABLE auth.users
    ADD CONSTRAINT auth_users_status_check
    CHECK (status IN (
        'pending_verification',
        'profile_incomplete',
        'pending_kyc',
        'rejected',
        'active',
        'suspended_temp',
        'suspended_permanent'
    ));

-- 1.4 Tabel riwayat penangguhan akun + audit
CREATE TABLE IF NOT EXISTS auth.account_suspension (
    id           UUID        NOT NULL DEFAULT gen_random_uuid() PRIMARY KEY,
    user_id      UUID        NOT NULL,            -- denormalized, no cross-schema FK
    is_permanent BOOLEAN     NOT NULL DEFAULT false,
    reason       TEXT        NOT NULL,
    expires_at   TIMESTAMPTZ,                      -- null = permanen
    created_by   UUID        NOT NULL,             -- admin pemicu (audit)
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_account_suspension_user_id
    ON auth.account_suspension (user_id);

-- 1.5 OTP: kolom attempts untuk batas percobaan verifikasi
ALTER TABLE auth.otp_verifications
    ADD COLUMN IF NOT EXISTS attempts INT NOT NULL DEFAULT 0;
