#[derive(Debug, thiserror::Error)]
pub enum SchedulerError {
    #[error("scheduler tidak terkonfigurasi (REDIS_URL kosong/invalid)")]
    Unconfigured,
    #[error("redis error: {0}")]
    Redis(#[from] redis::RedisError),
    #[error("gagal serialize/deserialize job envelope: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("tidak ada handler terdaftar untuk job_type '{0}'")]
    HandlerNotFound(String),
    #[error("handler gagal memproses job: {0}")]
    HandlerFailed(String),
}
