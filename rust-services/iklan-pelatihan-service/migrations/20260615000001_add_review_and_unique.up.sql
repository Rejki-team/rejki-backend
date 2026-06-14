-- Add reviewed_by + review_note columns to iklan_pelatihan for moderation audit trail.
-- Ref: openspec/changes/add-pelatihan-enrollment-badge/design.md D1, D2

ALTER TABLE iklan_pelatihan.iklan
    ADD COLUMN IF NOT EXISTS reviewed_by UUID,
    ADD COLUMN IF NOT EXISTS review_note TEXT;

-- Add unique constraints to prevent duplicate enrollments and badges.
ALTER TABLE iklan_pelatihan.pelatihan_enrollment
    ADD CONSTRAINT IF NOT EXISTS uq_enrollment_user_pelatihan UNIQUE (pelatihan_id, user_id);

ALTER TABLE iklan_pelatihan.pelatihan_badge
    ADD CONSTRAINT IF NOT EXISTS uq_badge_user_pelatihan UNIQUE (pelatihan_id, user_id);
