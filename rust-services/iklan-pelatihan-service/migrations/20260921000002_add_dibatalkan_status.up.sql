-- Tambah status 'dibatalkan' (terminal) — tugas otomatis Bab 10 P6.4 (F-32,
-- Kelompok 2 Phase 6): H-8 jam sebelum mulai, masih verifikasi → batalkan otomatis.
-- Non-destruktif: hanya memperluas CHECK constraint, tidak menghapus nilai lama.

ALTER TABLE iklan_pelatihan.iklan
    DROP CONSTRAINT IF EXISTS iklan_status_check;

ALTER TABLE iklan_pelatihan.iklan
    ADD CONSTRAINT iklan_status_check CHECK (status IN (
        'verifikasi_tertunda',
        'verifikasi_dalam_proses',
        'verifikasi_ditolak',
        'verifikasi_diterima',
        'pelatihan_belum_dimulai',
        'pelatihan_berjalan',
        'pelatihan_selesai',
        'dibatalkan'
    ));
