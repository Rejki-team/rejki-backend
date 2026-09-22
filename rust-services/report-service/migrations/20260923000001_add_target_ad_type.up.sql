-- P9.0 (Kelompok 6, Q9): `target_type` hanya bisa bedakan Iklan/User — TIDAK
-- bisa bedakan Iklan Pekerjaan/Pekerja/Barang Bekas satu sama lain. Kolom ini
-- menutup gap itu supaya endpoint approve-and-suspend (P9.1) tahu domain
-- client mana yang harus dipanggil. Nullable: NULL bila target_type=User atau
-- tidak diketahui (baris existing).
ALTER TABLE report.report
    ADD COLUMN target_ad_type TEXT
        CHECK (target_ad_type IN ('pekerjaan', 'pekerja', 'barang_bekas'));

CREATE INDEX idx_report_target_ad_type ON report.report (target_ad_type);
