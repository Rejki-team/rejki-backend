-- Rollback: gratis → jual-beli.

DROP INDEX IF EXISTS iklan_barang_bekas.idx_barang_bekas_created;

ALTER TABLE iklan_barang_bekas.iklan
    DROP COLUMN IF EXISTS jenis_barang,
    DROP COLUMN IF EXISTS jumlah,
    DROP COLUMN IF EXISTS lokasi_pengambilan,
    DROP COLUMN IF EXISTS availability_status,
    ADD COLUMN IF NOT EXISTS harga      BIGINT  NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS kondisi    TEXT    NOT NULL DEFAULT 'bekas',
    ADD COLUMN IF NOT EXISTS is_sold    BOOLEAN NOT NULL DEFAULT false;

CREATE INDEX IF NOT EXISTS idx_barang_bekas_created
    ON iklan_barang_bekas.iklan (created_at DESC) WHERE is_sold = false;
