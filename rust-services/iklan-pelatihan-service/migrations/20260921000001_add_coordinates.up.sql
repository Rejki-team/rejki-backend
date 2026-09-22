-- Kolom koordinat iklan (F-1) — filter radius 10km (bukan region_id lama, keputusan B-1 PRD v1.4+).
-- Nullable dulu: baris existing belum ter-geocode, diisi belakangan oleh GeocodingClient (Phase 2).
ALTER TABLE iklan_pelatihan.iklan
    ADD COLUMN IF NOT EXISTS latitude  DOUBLE PRECISION,
    ADD COLUMN IF NOT EXISTS longitude DOUBLE PRECISION;

CREATE INDEX IF NOT EXISTS idx_iklan_pelatihan_coordinates ON iklan_pelatihan.iklan (latitude, longitude);
