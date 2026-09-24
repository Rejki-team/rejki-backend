-- Rollback: kembalikan CHECK constraint ke 7 status semula (tanpa 'dibatalkan').
-- Catatan: bila ada baris dengan status='dibatalkan', ALTER ini akan gagal (sengaja —
-- rollback destruktif terhadap data harus eksplisit, bukan diam-diam UPDATE/DELETE).

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
        'pelatihan_selesai'
    ));
