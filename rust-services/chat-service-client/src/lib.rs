use uuid::Uuid;

/// `#[async_trait]` wajib (bukan native `async fn in trait`) — trait ini dipakai sebagai
/// `Arc<dyn ChatClient>` oleh `iklan-pekerjaan-service`/`iklan-barang-bekas-service` (F-19,
/// Kelompok 4 Phase 4), native async-fn-in-trait tidak dyn-compatible. Pola sama
/// `IklanPekerjaanClient`/`IklanPekerjaClient`.
#[async_trait::async_trait]
pub trait ChatClient: Send + Sync {
    async fn get_conversation_id(
        &self,
        user_a: Uuid,
        user_b: Uuid,
    ) -> Result<Uuid, ChatClientError>;
    async fn conversation_exists(&self, conversation_id: Uuid) -> Result<bool, ChatClientError>;

    /// P4.3 (F-19, PRD §5.9/Bab 10): jadwalkan auto-end (+2x24 jam) untuk seluruh
    /// conversation aktif (`ended_at IS NULL`) yang terhubung ke iklan ini — dipanggil
    /// service lain setelah "proses pada iklan terkait selesai" (mis. Lamaran ditandai
    /// selesai, Bider disetujui). Fail-open di sisi pemanggil — kegagalan di sini TIDAK
    /// boleh menggagalkan operasi utama pemanggil (§4.5 backend, I/O aman).
    async fn schedule_auto_end_for_ad(
        &self,
        ad_type: &str,
        ad_id: Uuid,
    ) -> Result<(), ChatClientError>;
}

#[derive(Debug, thiserror::Error)]
pub enum ChatClientError {
    #[error("conversation not found")]
    NotFound,
    #[error("chat service unavailable")]
    Unavailable,
}
