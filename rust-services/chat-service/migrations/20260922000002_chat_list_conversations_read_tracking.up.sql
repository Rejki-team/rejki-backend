-- F-18/F-19 (P4.10, P4.11) — fondasi "Halaman daftar percakapan" (PRD §5.9): daftar
-- percakapan milik user (P4.10) butuh cara mengetahui "sudah dibaca sampai mana" per
-- participant untuk indikator belum dibaca (P4.11). Kolom langsung di `conversations`
-- (bukan tabel participant terpisah) — konsisten dengan `user_a`/`user_b` denormalized
-- yang sudah ada (skema chat memang "tepat 2 pihak per conversation", bukan N pihak).

ALTER TABLE chat.conversations
    ADD COLUMN user_a_last_read_message_id UUID REFERENCES chat.messages (id) ON DELETE SET NULL,
    ADD COLUMN user_b_last_read_message_id UUID REFERENCES chat.messages (id) ON DELETE SET NULL;
