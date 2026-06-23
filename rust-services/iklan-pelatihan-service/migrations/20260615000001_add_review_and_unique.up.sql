-- Add reviewed_by + review_note columns to iklan_pelatihan for moderation audit trail.
-- Ref: openspec/changes/add-pelatihan-enrollment-badge/design.md D1, D2

ALTER TABLE iklan_pelatihan.iklan
    ADD COLUMN IF NOT EXISTS reviewed_by UUID,
    ADD COLUMN IF NOT EXISTS review_note TEXT;

-- Add unique constraints to prevent duplicate enrollments and badges.
-- Use DO block because ADD CONSTRAINT IF NOT EXISTS is not supported in PG.
DO $$ BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'uq_enrollment_user_pelatihan') THEN
    ALTER TABLE iklan_pelatihan.pelatihan_enrollment
      ADD CONSTRAINT uq_enrollment_user_pelatihan UNIQUE (pelatihan_id, user_id);
  END IF;
END $$;

DO $$ BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'uq_badge_user_pelatihan') THEN
    ALTER TABLE iklan_pelatihan.pelatihan_badge
      ADD CONSTRAINT uq_badge_user_pelatihan UNIQUE (pelatihan_id, user_id);
  END IF;
END $$;
