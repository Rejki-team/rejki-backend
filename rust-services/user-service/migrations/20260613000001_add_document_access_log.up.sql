-- Audit trail akses dokumen KYC (Open Question Q2).
-- Setiap akses dokumen wajib terlacak: kapan diupload, kapan dibuka/dilihat,
-- kapan presigned diterbitkan — termasuk presigned yang dipakai pengguna reguler.
-- Tabel append-only (tidak ada UPDATE/DELETE di kode) demi keamanan audit & sengketa.
-- Ref: openspec/changes/add-user-service-kyc/specs/kyc-documents

CREATE TABLE IF NOT EXISTS user_svc.document_access_log (
    id          UUID        NOT NULL DEFAULT gen_random_uuid() PRIMARY KEY,
    actor_id    UUID        NOT NULL,          -- siapa yang mengakses (pemilik/admin)
    object_key  TEXT        NOT NULL,          -- dokumen mana (object key storage)
    action      TEXT        NOT NULL,          -- 'upload_issued' | 'commit' | 'read_issued'
    request_id  TEXT,                          -- korelasi request (propagasi request_id)
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_doc_access_actor
    ON user_svc.document_access_log (actor_id);
CREATE INDEX IF NOT EXISTS idx_doc_access_object
    ON user_svc.document_access_log (object_key);
