-- Kelompok 3 Phase 3 (F-15): entity Bider — daftar peminat Iklan Barang Bekas
-- (PRD §5.14.1-5.14.2, Gambar 5). Tabel baru, tidak menyentuh kolom `iklan` existing.

CREATE TABLE IF NOT EXISTS iklan_barang_bekas.bider (
    id                UUID        NOT NULL DEFAULT gen_random_uuid() PRIMARY KEY,
    iklan_id          UUID        NOT NULL,
    peminat_id        UUID        NOT NULL,
    status            TEXT        NOT NULL DEFAULT 'menunggu'
        CHECK (status IN ('menunggu', 'disetujui', 'withdrawn')),
    sudah_menghubungi BOOLEAN     NOT NULL DEFAULT false,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_bider_iklan_id ON iklan_barang_bekas.bider (iklan_id);
CREATE INDEX IF NOT EXISTS idx_bider_peminat_id ON iklan_barang_bekas.bider (peminat_id);
-- Cek bid aktif ganda (P3.2 validasi): filter per iklan+peminat, status masih menunggu.
CREATE INDEX IF NOT EXISTS idx_bider_iklan_peminat_pending
    ON iklan_barang_bekas.bider (iklan_id, peminat_id)
    WHERE status = 'menunggu';
