CREATE SCHEMA IF NOT EXISTS iklan_pekerjaan;

CREATE TABLE iklan_pekerjaan.iklan (
    id         UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    poster_id  UUID        NOT NULL,
    judul      TEXT        NOT NULL,
    perusahaan TEXT        NOT NULL,
    deskripsi  TEXT        NOT NULL DEFAULT '',
    lokasi     TEXT,
    gaji_min   BIGINT,
    gaji_max   BIGINT,
    tipe       TEXT        NOT NULL DEFAULT 'full_time',
    is_active  BOOLEAN     NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_pekerjaan_poster  ON iklan_pekerjaan.iklan (poster_id);
CREATE INDEX idx_pekerjaan_tipe    ON iklan_pekerjaan.iklan (tipe) WHERE is_active = true;
CREATE INDEX idx_pekerjaan_created ON iklan_pekerjaan.iklan (created_at DESC) WHERE is_active = true;
