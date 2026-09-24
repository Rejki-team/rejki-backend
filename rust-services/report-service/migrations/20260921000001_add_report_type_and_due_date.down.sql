DROP INDEX IF EXISTS report.idx_report_due_date_active;
DROP INDEX IF EXISTS report.idx_report_report_type;
ALTER TABLE report.report ALTER COLUMN target_id SET NOT NULL;
ALTER TABLE report.report ALTER COLUMN target_type SET NOT NULL;
ALTER TABLE report.report DROP COLUMN due_date;
ALTER TABLE report.report DROP COLUMN report_type;
