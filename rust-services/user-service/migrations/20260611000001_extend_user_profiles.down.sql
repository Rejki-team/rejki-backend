DROP TABLE IF EXISTS user_svc.kyc_submission;

ALTER TABLE user_svc.profiles
    DROP COLUMN IF EXISTS full_name,
    DROP COLUMN IF EXISTS nik_encrypted,
    DROP COLUMN IF EXISTS nik_last4,
    DROP COLUMN IF EXISTS education_level,
    DROP COLUMN IF EXISTS gender,
    DROP COLUMN IF EXISTS birth_date,
    DROP COLUMN IF EXISTS address_line,
    DROP COLUMN IF EXISTS country_code,
    DROP COLUMN IF EXISTS province_id,
    DROP COLUMN IF EXISTS regency_id,
    DROP COLUMN IF EXISTS district_id,
    DROP COLUMN IF EXISTS village_id;
