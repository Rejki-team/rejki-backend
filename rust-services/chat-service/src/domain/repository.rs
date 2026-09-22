use uuid::Uuid;

use super::entity::{Conversation, Message, NewMessage};

/// `async fn` di trait kini stabil; lint hanya menyoroti ketiadaan Send bound otomatis.
/// Repository dipakai in-process; cukup di-allow.
#[allow(async_fn_in_trait)]
pub trait ChatRepository: Send + Sync {
    /// Mengembalikan conversation + apakah baris ini BARU dibuat (bukan sudah ada
    /// sebelumnya, dideteksi via `xmax = 0`) — dipakai untuk menjadwalkan retensi
    /// 60 hari (P4.5) hanya sekali per conversation, bukan di setiap panggilan.
    /// `related_ad_type`/`related_ad_id` opsional — bila diisi, MENIMPA link lama
    /// (percakapan terbaru dianggap konteks yang relevan; keterbatasan skema
    /// "satu conversation per pasangan user", dicatat sebagai gap di plan).
    async fn find_or_create_conversation(
        &self,
        user_a: Uuid,
        user_b: Uuid,
        related_ad_type: Option<&str>,
        related_ad_id: Option<Uuid>,
    ) -> Result<(Conversation, bool), anyhow::Error>;

    async fn find_conversation_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<Conversation>, anyhow::Error>;

    async fn conversation_exists(&self, id: Uuid) -> Result<bool, anyhow::Error>;

    /// Membership check (WS P4.0 & REST) — IDOR-safe: `user_id` harus `user_a`/`user_b`.
    async fn is_participant(
        &self,
        conversation_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, anyhow::Error>;

    async fn save_message(
        &self,
        conversation_id: Uuid,
        sender_id: Uuid,
        input: &NewMessage,
    ) -> Result<Message, anyhow::Error>;

    async fn list_messages(
        &self,
        conversation_id: Uuid,
        limit: i64,
        before_id: Option<Uuid>,
        after_id: Option<Uuid>,
    ) -> Result<Vec<Message>, anyhow::Error>;

    /// P4.4 — akhiri manual. Ownership check di query (IDOR→404): hanya participant.
    /// Idempoten: no-op (return `None`) bila sudah `ended_at IS NOT NULL`.
    async fn end_conversation(
        &self,
        id: Uuid,
        actor_id: Uuid,
    ) -> Result<Option<Conversation>, anyhow::Error>;

    /// P4.3 — dipanggil job handler `chat_auto_end_conversation`. Idempoten: no-op
    /// bila sudah berakhir (manual atau auto-end sebelumnya).
    async fn auto_end_conversation(&self, id: Uuid) -> Result<bool, anyhow::Error>;

    /// P4.3 — fan-out trigger: seluruh conversation AKTIF (`ended_at IS NULL`) yang
    /// terhubung ke satu iklan (dipanggil `ChatClient::schedule_auto_end_for_ad`).
    async fn find_active_conversations_by_ad(
        &self,
        ad_type: &str,
        ad_id: Uuid,
    ) -> Result<Vec<Conversation>, anyhow::Error>;

    /// P4.5 — hard delete (cascade ke messages via FK). Idempoten: `false` bila
    /// conversation sudah tidak ada (job retensi lain/manual sudah menghapusnya).
    async fn purge_conversation(&self, id: Uuid) -> Result<bool, anyhow::Error>;

    /// P4.10 — daftar conversation milik `user_id` (urut aktivitas terbaru), masing-masing
    /// beserta pesan terakhirnya (`None` bila belum ada pesan sama sekali).
    async fn list_conversations_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<(Conversation, Option<Message>)>, anyhow::Error>;

    /// P4.11 — tandai `user_id` sudah membaca sampai pesan terakhir di conversation ini.
    /// Ownership check di query (IDOR→404) — `false` bila `id` tidak ada atau `user_id`
    /// bukan participant.
    async fn mark_read(&self, id: Uuid, user_id: Uuid) -> Result<bool, anyhow::Error>;
}
