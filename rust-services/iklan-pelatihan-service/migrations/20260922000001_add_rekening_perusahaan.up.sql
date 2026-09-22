-- Nomor rekening perusahaan penyelenggara (F-9, keputusan final klien B-5b) —
-- rekening tujuan transfer biaya komitmen peserta. Nullable dulu sesuai
-- konvensi non-destruktif — pelatihan lama tanpa rekening tetap valid
-- historis; wajib diisi untuk pengajuan BARU divalidasi di application layer
-- (CreateIklanPelatihanInput), bukan CHECK constraint DB.
-- Nama kolom mengikuti field yang SUDAH dikirim mobile hari ini
-- (`training_remote_data_source.dart`) — bukan usulan awal
-- nomor_rekening/bank/nama_pemilik_rekening — supaya konsisten saat kontrak
-- mobile diperbaiki di task terpisah (lihat plan Kelompok 6 P1.0).

ALTER TABLE iklan_pelatihan.iklan
    ADD COLUMN bank_name TEXT,
    ADD COLUMN bank_account_number TEXT,
    ADD COLUMN bank_account_holder_name TEXT;
