-- Rollback reviewed_by, review_note, and unique constraints.

ALTER TABLE iklan_pelatihan.pelatihan_badge
    DROP CONSTRAINT IF EXISTS uq_badge_user_pelatihan;

ALTER TABLE iklan_pelatihan.pelatihan_enrollment
    DROP CONSTRAINT IF EXISTS uq_enrollment_user_pelatihan;

ALTER TABLE iklan_pelatihan.iklan
    DROP COLUMN IF EXISTS reviewed_by,
    DROP COLUMN IF EXISTS review_note;
