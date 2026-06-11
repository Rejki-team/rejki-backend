-- Rollback extend auth onboarding

ALTER TABLE auth.otp_verifications DROP COLUMN IF EXISTS attempts;

DROP TABLE IF EXISTS auth.account_suspension;

-- Kembalikan is_verified dari status sebelum drop kolom status
ALTER TABLE auth.users ADD COLUMN IF NOT EXISTS is_verified BOOLEAN NOT NULL DEFAULT false;
UPDATE auth.users SET is_verified = true
    WHERE status IN ('profile_incomplete', 'pending_kyc', 'rejected', 'active', 'suspended_temp', 'suspended_permanent');

ALTER TABLE auth.users DROP CONSTRAINT IF EXISTS auth_users_status_check;

ALTER TABLE auth.users
    DROP COLUMN IF EXISTS phone,
    DROP COLUMN IF EXISTS status,
    DROP COLUMN IF EXISTS tos_accepted_at,
    DROP COLUMN IF EXISTS tos_version;
