ALTER TABLE iklan_pelatihan.iklan
    DROP COLUMN IF EXISTS bank_name,
    DROP COLUMN IF EXISTS bank_account_number,
    DROP COLUMN IF EXISTS bank_account_holder_name;
