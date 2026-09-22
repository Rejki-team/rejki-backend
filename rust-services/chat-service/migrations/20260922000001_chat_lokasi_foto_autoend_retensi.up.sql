-- F-19 (PRD §5.9/Bab 10, Kelompok 4 Phase 4): dukungan lokasi/foto di pesan,
-- linkage conversation<->iklan (fondasi auto-end), dan flag auto-end manual/otomatis.

-- messages: content_type Text/Location/Photo (kolom terpisah, bukan JSONB — tetap
-- bisa parameterized query & CHECK constraint sederhana).
ALTER TABLE chat.messages
    ADD COLUMN content_type TEXT NOT NULL DEFAULT 'text',
    ADD COLUMN lat DOUBLE PRECISION,
    ADD COLUMN lng DOUBLE PRECISION,
    ADD COLUMN photo_object_key TEXT;

-- content tidak lagi wajib — Location/Photo tidak mengisi content (tanpa caption,
-- keputusan desain: kesederhanaan, PRD tidak minta caption untuk lokasi/foto).
ALTER TABLE chat.messages ALTER COLUMN content DROP NOT NULL;

ALTER TABLE chat.messages
    ADD CONSTRAINT chk_messages_content_type CHECK (content_type IN ('text', 'location', 'photo'));

-- Defense in depth — field yang terisi harus persis sesuai content_type (pola sama
-- CHECK report_type di Kelompok 4 Phase 1).
ALTER TABLE chat.messages
    ADD CONSTRAINT chk_messages_content_matches_type CHECK (
        (content_type = 'text' AND content IS NOT NULL AND lat IS NULL AND lng IS NULL AND photo_object_key IS NULL)
        OR (content_type = 'location' AND lat IS NOT NULL AND lng IS NOT NULL AND content IS NULL AND photo_object_key IS NULL)
        OR (content_type = 'photo' AND photo_object_key IS NOT NULL AND content IS NULL AND lat IS NULL AND lng IS NULL)
    );

-- FK yang seharusnya sudah ada sejak awal (belum ada sebelumnya) — hard-delete retensi
-- 60 hari (P4.5) butuh ini supaya DELETE conversations otomatis membersihkan messages.
ALTER TABLE chat.messages
    ADD CONSTRAINT fk_messages_conversation FOREIGN KEY (conversation_id)
        REFERENCES chat.conversations (id) ON DELETE CASCADE;

-- conversations: akhir percakapan (manual/otomatis) + linkage opsional ke iklan terkait
-- (fondasi auto-end P4.3 — "proses pada iklan terkait selesai").
ALTER TABLE chat.conversations
    ADD COLUMN ended_at TIMESTAMPTZ,
    ADD COLUMN related_ad_type TEXT,
    ADD COLUMN related_ad_id UUID;

ALTER TABLE chat.conversations
    ADD CONSTRAINT chk_conversations_related_ad_type
        CHECK (related_ad_type IS NULL OR related_ad_type IN ('pekerjaan', 'pekerja', 'barang_bekas'));

CREATE INDEX IF NOT EXISTS idx_conversations_related_ad
    ON chat.conversations (related_ad_type, related_ad_id)
    WHERE related_ad_id IS NOT NULL;

-- Retensi P4.5 dijadwalkan dari created_at — index untuk audit/manual query.
CREATE INDEX IF NOT EXISTS idx_conversations_created_at ON chat.conversations (created_at);
