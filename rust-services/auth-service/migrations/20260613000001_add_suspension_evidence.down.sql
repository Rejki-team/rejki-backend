-- Rollback bukti penangguhan akun (Q1).
ALTER TABLE auth.account_suspension
    DROP COLUMN IF EXISTS evidence_object_key;
