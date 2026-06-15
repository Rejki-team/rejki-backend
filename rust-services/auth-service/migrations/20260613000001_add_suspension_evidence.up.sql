-- Bukti penangguhan akun (US-07 / Open Question Q1).
-- Admin yang men-suspend akun (sementara/permanen) wajib menyertakan alasan + bukti
-- (gambar/dokumen, maks 5MB per dokumen) demi keamanan audit & sengketa.
-- Object key merujuk ke objek di storage (kategori 'suspension-evidence'), bukan URL publik.
-- Ref: openspec/changes/add-user-service-kyc/design.md (Q1 resolved)

ALTER TABLE auth.account_suspension
    ADD COLUMN IF NOT EXISTS evidence_object_key TEXT;
