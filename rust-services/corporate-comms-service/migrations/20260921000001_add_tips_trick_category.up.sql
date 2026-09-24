ALTER TABLE comms.corporate_article DROP CONSTRAINT IF EXISTS corporate_article_category_check;

ALTER TABLE comms.corporate_article
    ADD CONSTRAINT corporate_article_category_check CHECK (category IN ('informasi', 'tips_trick'));
