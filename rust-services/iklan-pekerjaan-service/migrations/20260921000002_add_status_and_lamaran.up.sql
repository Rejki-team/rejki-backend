-- Kelompok 3 Phase 1 (F-3): entity Lamaran + status siklus kerja Iklan Pekerjaan.
-- Non-destruktif: kolom baru dulu (default 'tersedia', backfill semua baris existing),
-- `is_active` (toggle visibilitas pemilik) TETAP terpisah, tidak dihapus/digabung.

ALTER TABLE iklan_pekerjaan.iklan
    ADD COLUMN IF NOT EXISTS status TEXT NOT NULL DEFAULT 'tersedia'
        CHECK (status IN ('tersedia', 'sedang_dikerjakan', 'selesai'));

CREATE INDEX IF NOT EXISTS idx_pekerjaan_status
    ON iklan_pekerjaan.iklan (status)
    WHERE deleted_at IS NULL;

-- Backfill: seluruh baris existing belum pernah punya Lamaran (fitur baru) → semua 'tersedia'.
UPDATE iklan_pekerjaan.iklan SET status = 'tersedia' WHERE status IS NULL;

CREATE TABLE IF NOT EXISTS iklan_pekerjaan.lamaran (
    id             UUID        NOT NULL DEFAULT gen_random_uuid() PRIMARY KEY,
    iklan_id       UUID        NOT NULL,
    pelamar_id     UUID        NOT NULL,
    status         TEXT        NOT NULL DEFAULT 'diajukan'
        CHECK (status IN ('diajukan', 'diterima', 'ditolak', 'proses', 'selesai')),
    tanggal        DATE        NOT NULL,
    jam_mulai      TIME        NOT NULL,
    jam_akhir      TIME        NOT NULL,
    kuota_diambil  INTEGER     NOT NULL DEFAULT 1,
    alasan_batal   TEXT,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_lamaran_iklan_id ON iklan_pekerjaan.lamaran (iklan_id);
CREATE INDEX IF NOT EXISTS idx_lamaran_pelamar_id ON iklan_pekerjaan.lamaran (pelamar_id);
-- Cek bentrok jadwal (P1.3 validasi #3): filter per pelamar + rentang tanggal cepat.
CREATE INDEX IF NOT EXISTS idx_lamaran_pelamar_tanggal
    ON iklan_pekerjaan.lamaran (pelamar_id, tanggal)
    WHERE status IN ('diterima', 'proses');
