CREATE SCHEMA IF NOT EXISTS iklan_barang_bekas;

CREATE TABLE iklan_barang_bekas.iklan (
    id         UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    seller_id  UUID        NOT NULL,
    judul      TEXT        NOT NULL,
    deskripsi  TEXT        NOT NULL DEFAULT '',
    harga      BIGINT      NOT NULL DEFAULT 0,
    kondisi    TEXT        NOT NULL,
    lokasi     TEXT,
    foto_urls  TEXT[]      NOT NULL DEFAULT '{}',
    is_sold    BOOLEAN     NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_barang_bekas_seller   ON iklan_barang_bekas.iklan (seller_id);
CREATE INDEX idx_barang_bekas_created  ON iklan_barang_bekas.iklan (created_at DESC) WHERE is_sold = false;
