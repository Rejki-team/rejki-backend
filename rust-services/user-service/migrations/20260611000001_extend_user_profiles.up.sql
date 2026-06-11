-- Extend user_svc.profiles untuk onboarding KYC Phase 2
-- Ref: openspec/changes/add-user-service-kyc

ALTER TABLE user_svc.profiles
    ADD COLUMN IF NOT EXISTS full_name       TEXT,
    ADD COLUMN IF NOT EXISTS nik_encrypted   BYTEA,           -- AES-256-GCM, BUKAN plaintext
    ADD COLUMN IF NOT EXISTS nik_last4       TEXT,            -- 4 digit terakhir untuk tampilan ter-mask
    ADD COLUMN IF NOT EXISTS education_level TEXT,
    ADD COLUMN IF NOT EXISTS gender          TEXT,            -- male / female
    ADD COLUMN IF NOT EXISTS birth_date      DATE,
    ADD COLUMN IF NOT EXISTS address_line    TEXT,
    ADD COLUMN IF NOT EXISTS country_code    TEXT NOT NULL DEFAULT 'ID',
    ADD COLUMN IF NOT EXISTS province_id     TEXT,            -- kode wilayah dari region-service
    ADD COLUMN IF NOT EXISTS regency_id      TEXT,
    ADD COLUMN IF NOT EXISTS district_id     TEXT,
    ADD COLUMN IF NOT EXISTS village_id      TEXT;

-- Tabel submission KYC — satu baris per pengajuan, reviewer manusia/sistem
CREATE TABLE IF NOT EXISTS user_svc.kyc_submission (
    id                 UUID        NOT NULL DEFAULT gen_random_uuid() PRIMARY KEY,
    profile_id         UUID        NOT NULL,
    status             TEXT        NOT NULL DEFAULT 'pending', -- pending / approved / rejected
    ktp_object_key     TEXT,        -- path objek di object storage (bukan URL publik)
    selfie_object_key  TEXT,
    reviewed_by        UUID,        -- admin; null bila sistem
    review_note        TEXT,        -- alasan penolakan
    reviewed_at        TIMESTAMPTZ,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_kyc_submission_profile ON user_svc.kyc_submission (profile_id);
CREATE INDEX IF NOT EXISTS idx_kyc_submission_status  ON user_svc.kyc_submission (status);
