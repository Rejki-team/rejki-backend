-- Schema & tabel data wilayah administratif Indonesia 4 tingkat.
-- Kode wilayah sebagai PK (mis. "32", "32.04", "32.04.05", "32.04.05.2001").

CREATE SCHEMA IF NOT EXISTS region;

CREATE TABLE IF NOT EXISTS region.province (
    id   TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS region.regency (
    id          TEXT NOT NULL PRIMARY KEY,
    province_id TEXT NOT NULL REFERENCES region.province(id),
    name        TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_regency_province ON region.regency (province_id);

CREATE TABLE IF NOT EXISTS region.district (
    id         TEXT NOT NULL PRIMARY KEY,
    regency_id TEXT NOT NULL REFERENCES region.regency(id),
    name       TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_district_regency ON region.district (regency_id);

CREATE TABLE IF NOT EXISTS region.village (
    id          TEXT NOT NULL PRIMARY KEY,
    district_id TEXT NOT NULL REFERENCES region.district(id),
    name        TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_village_district ON region.village (district_id);

-- Versi dataset untuk audit/tracking update berkala.
CREATE TABLE IF NOT EXISTS region.dataset_version (
    source    TEXT NOT NULL,
    version   TEXT NOT NULL,
    seed_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
