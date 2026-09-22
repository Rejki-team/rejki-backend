use std::collections::HashMap;
use std::sync::Arc;

use crate::error::SchedulerError;

/// Handler untuk satu jenis tugas terjadwal (`job_type`). Setiap tugas Bab 10 PRD
/// (Kelompok 2 Phase 6) mengimplementasikan trait ini sekali dan mendaftar ke
/// `SchedulerRegistry` di composition root (`rejki-app`), bukan menulis consumer sendiri.
#[async_trait::async_trait]
pub trait JobHandler: Send + Sync {
    /// Proses satu job. Error → di-retry oleh `SchedulerConsumer` (exponential backoff,
    /// maks 3x) lalu masuk DLQ bila tetap gagal — implementasi TIDAK perlu retry sendiri.
    async fn handle(&self, payload: &serde_json::Value) -> Result<(), anyhow::Error>;
}

/// Registry `job_type` → handler. Dibangun sekali di composition root, dipakai
/// `SchedulerConsumer` untuk dispatch tiap job yang masuk dari stream.
#[derive(Clone, Default)]
pub struct SchedulerRegistry {
    handlers: HashMap<String, Arc<dyn JobHandler>>,
}

impl SchedulerRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Daftarkan handler untuk `job_type`. Memanggil dua kali dengan `job_type` yang
    /// sama akan menimpa (menghindari "Zero Injection Duplication" — pastikan tiap
    /// job_type hanya didaftarkan sekali di composition root).
    pub fn register(&mut self, job_type: impl Into<String>, handler: Arc<dyn JobHandler>) {
        self.handlers.insert(job_type.into(), handler);
    }

    pub fn get(&self, job_type: &str) -> Result<Arc<dyn JobHandler>, SchedulerError> {
        self.handlers
            .get(job_type)
            .cloned()
            .ok_or_else(|| SchedulerError::HandlerNotFound(job_type.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EchoHandler;

    #[async_trait::async_trait]
    impl JobHandler for EchoHandler {
        async fn handle(&self, _payload: &serde_json::Value) -> Result<(), anyhow::Error> {
            Ok(())
        }
    }

    #[test]
    fn test_registry_given_registered_handler_when_get_then_found() {
        let mut registry = SchedulerRegistry::new();
        registry.register("noop", Arc::new(EchoHandler));
        assert!(registry.get("noop").is_ok());
    }

    #[test]
    fn test_registry_given_unregistered_job_type_when_get_then_handler_not_found() {
        let registry = SchedulerRegistry::new();
        let is_not_found = matches!(
            registry.get("unknown"),
            Err(SchedulerError::HandlerNotFound(ref t)) if t == "unknown"
        );
        assert!(is_not_found, "expected HandlerNotFound(\"unknown\")");
    }
}
