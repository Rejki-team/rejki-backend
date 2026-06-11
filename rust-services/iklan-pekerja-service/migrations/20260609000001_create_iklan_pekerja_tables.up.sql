CREATE SCHEMA IF NOT EXISTS iklan_pekerja;

CREATE TABLE iklan_pekerja.iklan (
    id         UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    poster_id  UUID        NOT NULL,
    nama       TEXT        NOT NULL,
    keahlian   TEXT[]      NOT NULL DEFAULT '{}',
    deskripsi  TEXT        NOT NULL DEFAULT '',
    lokasi     TEXT,
    tarif_min  BIGINT,
    tarif_max  BIGINT,
    is_active  BOOLEAN     NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_pekerja_poster   ON iklan_pekerja.iklan (poster_id);
CREATE INDEX idx_pekerja_created  ON iklan_pekerja.iklan (created_at DESC) WHERE is_active = true;
CREATE INDEX idx_pekerja_keahlian ON iklan_pekerja.iklan USING GIN (keahlian) WHERE is_active = true;
