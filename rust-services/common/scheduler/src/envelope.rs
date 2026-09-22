use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Payload satu tugas terjadwal — dibungkus di setiap tahap (sorted-set delay
/// buffer → stream → DLQ) tanpa berubah bentuk (F-32).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobEnvelope {
    /// Unik per tugas — dipakai sebagai idempotency key (`scheduler:processed:<job_id>`)
    /// dan sebagai member unik di sorted-set delay buffer (dua job dengan `job_type`+
    /// `payload` identik tetap tidak akan saling menimpa).
    pub job_id: Uuid,
    /// Nama handler yang dipanggil `SchedulerRegistry` — mis. "kyc_purge_documents".
    pub job_type: String,
    /// Payload bebas (tergantung `job_type`) — divalidasi/deserialize oleh handler-nya sendiri.
    pub payload: serde_json::Value,
    /// Waktu tugas boleh dieksekusi. Dijadwalkan lewat sorted-set (score = unix timestamp
    /// detik) — TIDAK dieksekusi sebelum waktu ini (lihat `SchedulerClient::schedule`).
    pub execute_after: DateTime<Utc>,
}

impl JobEnvelope {
    pub fn new(
        job_type: impl Into<String>,
        payload: serde_json::Value,
        execute_after: DateTime<Utc>,
    ) -> Self {
        Self {
            job_id: Uuid::now_v7(),
            job_type: job_type.into(),
            payload,
            execute_after,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_envelope_given_new_when_serialized_then_roundtrips() {
        let env = JobEnvelope::new(
            "kyc_purge_documents",
            serde_json::json!({"profile_id": "abc"}),
            Utc::now(),
        );
        let json = serde_json::to_string(&env).unwrap();
        let back: JobEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(back.job_id, env.job_id);
        assert_eq!(back.job_type, "kyc_purge_documents");
    }
}
