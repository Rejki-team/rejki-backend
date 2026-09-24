-- Kelompok 3 Phase 5 (F-17): entity Rating dua arah (PRD §5.15).
-- Schema generik — TIDAK membedakan modul sumber (`iklan_id` bisa dari modul manapun
-- yang punya fitur rating nanti), agar agregasi P5.4 tidak perlu migrasi ulang skema.

CREATE SCHEMA IF NOT EXISTS rating;

CREATE TABLE IF NOT EXISTS rating.rating (
    id            UUID        NOT NULL DEFAULT gen_random_uuid() PRIMARY KEY,
    iklan_id      UUID        NOT NULL,
    penilai_id    UUID        NOT NULL,
    dinilai_id    UUID        NOT NULL,
    arah          TEXT        NOT NULL
        CHECK (arah IN ('pelamar_ke_pemberi_kerja', 'pemberi_kerja_ke_pelamar')),
    bintang       SMALLINT    NOT NULL CHECK (bintang BETWEEN 1 AND 5),
    ulasan        TEXT        CHECK (ulasan IS NULL OR char_length(ulasan) BETWEEN 20 AND 255),
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- PRD §5.15: satu kali per pasangan pengguna pada satu kode iklan.
CREATE UNIQUE INDEX IF NOT EXISTS uq_rating_pasangan_iklan
    ON rating.rating (iklan_id, penilai_id, dinilai_id);

-- Agregasi "rating keaktifan" (P5.4) — filter utama adalah dinilai_id.
CREATE INDEX IF NOT EXISTS idx_rating_dinilai_id ON rating.rating (dinilai_id);
