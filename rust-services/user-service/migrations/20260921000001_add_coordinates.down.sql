DROP INDEX IF EXISTS user_svc.idx_profiles_coordinates;

ALTER TABLE user_svc.profiles
    DROP COLUMN IF EXISTS latitude,
    DROP COLUMN IF EXISTS longitude;
