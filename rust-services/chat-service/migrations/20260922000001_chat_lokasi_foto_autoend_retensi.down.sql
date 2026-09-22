DROP INDEX IF EXISTS chat.idx_conversations_created_at;
DROP INDEX IF EXISTS chat.idx_conversations_related_ad;
ALTER TABLE chat.conversations DROP CONSTRAINT IF EXISTS chk_conversations_related_ad_type;
ALTER TABLE chat.conversations DROP COLUMN IF EXISTS related_ad_id;
ALTER TABLE chat.conversations DROP COLUMN IF EXISTS related_ad_type;
ALTER TABLE chat.conversations DROP COLUMN IF EXISTS ended_at;

ALTER TABLE chat.messages DROP CONSTRAINT IF EXISTS fk_messages_conversation;
ALTER TABLE chat.messages DROP CONSTRAINT IF EXISTS chk_messages_content_matches_type;
ALTER TABLE chat.messages DROP CONSTRAINT IF EXISTS chk_messages_content_type;
ALTER TABLE chat.messages ALTER COLUMN content SET NOT NULL;
ALTER TABLE chat.messages DROP COLUMN IF EXISTS photo_object_key;
ALTER TABLE chat.messages DROP COLUMN IF EXISTS lng;
ALTER TABLE chat.messages DROP COLUMN IF EXISTS lat;
ALTER TABLE chat.messages DROP COLUMN IF EXISTS content_type;
