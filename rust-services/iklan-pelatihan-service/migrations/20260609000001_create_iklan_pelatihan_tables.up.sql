CREATE SCHEMA IF NOT EXISTS iklan_pelatihan;

CREATE TABLE iklan_pelatihan.iklan (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    poster_id       UUID        NOT NULL,
    judul           TEXT        NOT NULL,
    penyelenggara   TEXT        NOT NULL,
    deskripsi       TEXT        NOT NULL DEFAULT '',
    lokasi          TEXT,
    harga           BIGINT,
    tanggal_mulai   TIMESTAMPTZ,
    tanggal_selesai TIMESTAMPTZ,
    is_active       BOOLEAN     NOT NULL DEFAULT true,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_pelatihan_poster   ON iklan_pelatihan.iklan (poster_id);
CREATE INDEX idx_pelatihan_created  ON iklan_pelatihan.iklan (created_at DESC) WHERE is_active = true;
CREATE INDEX idx_pelatihan_tanggal  ON iklan_pelatihan.iklan (tanggal_mulai) WHERE is_active = true;
