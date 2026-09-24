-- Tanda tangan pejabat perusahaan (F-11, dipakai generator sertifikat PDF
-- Phase 6) — field pada Iklan Pelatihan (satu tanda tangan berlaku untuk
-- SELURUH peserta pelatihan itu), BUKAN pada PelatihanBadge per-peserta.
-- `badge_icon_object_key` SENGAJA TIDAK jadi kolom DB — riset sebelum coding
-- menemukan mobile tidak pernah mengunggah field ini; diputuskan sebagai aset
-- branding tetap (bundled di mobile app), bukan data per-record (lihat plan
-- Kelompok 6 Phase 5, keputusan desain).

ALTER TABLE iklan_pelatihan.iklan
    ADD COLUMN signature_object_key TEXT;
