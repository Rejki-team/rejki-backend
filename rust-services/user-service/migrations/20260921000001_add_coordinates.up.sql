-- Kolom koordinat alamat pengguna (F-1) — pengganti filter berbasis region_id administratif
-- lama dengan radius kilometer (keputusan final klien B-1, PRD v1.4+). Nullable dulu: baris
-- existing belum ter-geocode, diisi belakangan oleh GeocodingClient (Phase 2) saat alamat
-- dibuat/diupdate. PostGIS TIDAK tersedia di image `postgres:17-alpine` yang dipakai (dibuktikan
-- `CREATE EXTENSION postgis` gagal) — pakai bounding-box + composite index + Haversine di query.
ALTER TABLE user_svc.profiles
    ADD COLUMN IF NOT EXISTS latitude  DOUBLE PRECISION,
    ADD COLUMN IF NOT EXISTS longitude DOUBLE PRECISION;

-- Composite index untuk bounding-box pre-filter (WHERE latitude BETWEEN .. AND longitude BETWEEN ..)
-- sebelum Haversine presisi — Hazard #6/#7, hindari full-table scan.
CREATE INDEX IF NOT EXISTS idx_profiles_coordinates ON user_svc.profiles (latitude, longitude);
