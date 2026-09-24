-- P1.1: Jenis Laporan (jalur pembuatan aduan — PRD §6.10, keputusan final B-8).
-- Default aman untuk baris existing (satu-satunya jalur yang sudah ada sebelumnya
-- adalah "Laporkan Iklan" generik).
ALTER TABLE report.report
    ADD COLUMN report_type TEXT NOT NULL DEFAULT 'laporkan_iklan'
        CHECK (report_type IN ('laporkan_iklan', 'pelaporan_masalah'));

-- P1.2: SLA 7 hari kerja. `due_date` dihitung sekali saat insert (lihat
-- `report_service::add_business_days`); `is_overdue` SENGAJA TIDAK dipersist —
-- dihitung on-the-fly (`due_date < now() AND status bukan terminal`), lebih
-- sederhana & selalu akurat tanpa job scheduler tambahan (keputusan dicatat di
-- plan). Baris existing diisi `created_at + 7 hari kalender` sebagai perkiraan
-- wajar (data lama tidak pernah dimaksudkan diukur SLA ini).
ALTER TABLE report.report
    ADD COLUMN due_date TIMESTAMPTZ;

UPDATE report.report SET due_date = created_at + INTERVAL '7 days' WHERE due_date IS NULL;

ALTER TABLE report.report ALTER COLUMN due_date SET NOT NULL;

-- Jalur "Pelaporan Masalah" boleh tidak menyasar iklan/user tertentu (gangguan
-- teknis pada proses) — PRD §6.10 "Kolom ID Iklan ... harus boleh kosong".
ALTER TABLE report.report ALTER COLUMN target_type DROP NOT NULL;
ALTER TABLE report.report ALTER COLUMN target_id DROP NOT NULL;

CREATE INDEX idx_report_report_type ON report.report (report_type);
-- Index parsial untuk query overdue (status aktif, due_date terlewat) — hindari scan penuh.
CREATE INDEX idx_report_due_date_active ON report.report (due_date)
    WHERE status NOT IN ('resolved', 'rejected');
