use std::sync::Arc;
use uuid::Uuid;

use super::dto::{
    AdminIklanPekerjaanResponse, AdminListQuery, CreateIklanPekerjaanInput, IklanPekerjaanResponse,
    ListQuery, SuspendEvidenceInput, SuspendInput, SuspendResponse, SuspendResultItem,
};
use crate::domain::repository::{AdminListParams, CreatePekerjaanParams, IklanPekerjaanRepository};
use common_rate_limit::RateLimiter;
use notification_service_client::NotificationClient;
use storage_service_client::StorageClient;

/// Default untuk pagination & storage.
const DEFAULT_LIMIT: i64 = 20;
const CSV_MAX: i64 = 10_000;
pub mod storage_category {
    pub const SUSPENSION_EVIDENCE: &str = "iklan-suspension-evidence";
}

pub struct IklanPekerjaanService<R: IklanPekerjaanRepository> {
    repo: Arc<R>,
    rate_limiter: Option<Arc<dyn RateLimiter>>,
}

impl<R: IklanPekerjaanRepository> IklanPekerjaanService<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self {
            repo,
            rate_limiter: None,
        }
    }

    pub fn with_rate_limiter(mut self, rl: Arc<dyn RateLimiter>) -> Self {
        self.rate_limiter = Some(rl);
        self
    }

    // ── Public endpoints ──────────────────────────────────────────────────────

    pub async fn list(
        &self,
        query: ListQuery,
    ) -> Result<Vec<IklanPekerjaanResponse>, anyhow::Error> {
        let items = self
            .repo
            .list(
                query.limit.unwrap_or(DEFAULT_LIMIT),
                query.offset.unwrap_or(0),
            )
            .await?;
        Ok(items.into_iter().map(to_response).collect())
    }

    pub async fn get(&self, id: Uuid) -> Result<IklanPekerjaanResponse, anyhow::Error> {
        self.repo
            .find_by_id(id)
            .await?
            .map(to_response)
            .ok_or_else(|| anyhow::anyhow!("iklan tidak ditemukan"))
    }

    pub async fn create(
        &self,
        poster_id: Uuid,
        input: CreateIklanPekerjaanInput,
    ) -> Result<IklanPekerjaanResponse, anyhow::Error> {
        // Rate limit: 30 req/15 menit per user untuk create iklan.
        if let Some(rl) = &self.rate_limiter {
            if !rl
                .allow("iklan_pekerjaan:create", &poster_id.to_string())
                .await
            {
                return Err(anyhow::anyhow!(
                    "terlalu banyak permintaan, coba lagi nanti"
                ));
            }
        }

        // Cek cooldown 3 hari — suspend permanen.
        if self.repo.is_poster_in_cooldown(poster_id).await? {
            return Err(anyhow::anyhow!(
                "Anda tidak dapat membuat iklan baru selama 3 hari setelah iklan ditangguhkan secara permanen"
            ));
        }
        let judul = ammonia::clean_text(&input.judul);
        let deskripsi = ammonia::clean_text(&input.deskripsi);
        let item = self
            .repo
            .create(CreatePekerjaanParams {
                poster_id,
                judul: &judul,
                perusahaan: &input.perusahaan,
                deskripsi: &deskripsi,
                tipe: &input.tipe,
                lokasi: input.lokasi.as_deref(),
                gaji_min: input.gaji_min,
                gaji_max: input.gaji_max,
            })
            .await?;
        Ok(to_response(item))
    }

    pub async fn delete(&self, id: Uuid, poster_id: Uuid) -> Result<bool, anyhow::Error> {
        self.repo.delete(id, poster_id).await
    }

    // ── Admin: listing ────────────────────────────────────────────────────────

    pub async fn admin_list(
        &self,
        query: AdminListQuery,
    ) -> Result<(Vec<AdminIklanPekerjaanResponse>, i64), anyhow::Error> {
        let params = AdminListParams {
            q: query.q,
            moderation_status: query.status,
            sort_by: query.sort_by,
            sort_dir: query.sort_dir,
            limit: query.limit.unwrap_or(DEFAULT_LIMIT),
            offset: query.offset.unwrap_or(0),
        };
        let result = self.repo.admin_list(params).await?;
        Ok((
            result.items.into_iter().map(to_admin_response).collect(),
            result.total,
        ))
    }

    // ── Admin: export CSV ─────────────────────────────────────────────────────

    pub async fn admin_export_csv(
        &self,
        query: AdminListQuery,
    ) -> Result<Vec<AdminIklanPekerjaanResponse>, anyhow::Error> {
        let params = AdminListParams {
            q: query.q,
            moderation_status: query.status,
            sort_by: query.sort_by,
            sort_dir: query.sort_dir,
            limit: CSV_MAX, // batas atas CSV
            offset: 0,
        };
        let items = self.repo.admin_list_all(params).await?;
        Ok(items.into_iter().map(to_admin_response).collect())
    }

    // ── Admin: evidence upload ────────────────────────────────────────────────

    pub async fn request_suspend_evidence(
        &self,
        storage: &dyn StorageClient,
        admin_id: Uuid,
        input: SuspendEvidenceInput,
    ) -> Result<storage_service_client::UploadPermission, anyhow::Error> {
        let info = storage_service_client::FileInfo {
            mime: input.mime,
            size_bytes: input.size_bytes,
        };
        storage
            .request_upload(storage_category::SUSPENSION_EVIDENCE, admin_id, info)
            .await
            .map_err(|e| anyhow::anyhow!("storage error: {e}"))
    }

    // ── Admin: suspend (single / bulk) ────────────────────────────────────────

    pub async fn suspend(
        &self,
        admin_id: Uuid,
        input: SuspendInput,
        notifier: Option<&dyn NotificationClient>,
        auth_client: Option<&dyn auth_service_client::AuthClient>,
    ) -> Result<SuspendResponse, anyhow::Error> {
        let _status = if input.is_permanent {
            "suspended_permanent"
        } else {
            "suspended_temp"
        };

        let suspensions = self
            .repo
            .suspend(
                &input.iklan_ids,
                input.is_permanent,
                &input.reason,
                Some(&input.evidence_object_key),
                input.expires_at,
                admin_id,
            )
            .await?;

        // Kumpulkan poster_id yang terdampak untuk notifikasi.
        let mut results = Vec::with_capacity(input.iklan_ids.len());
        for iklan_id in &input.iklan_ids {
            let suspended = suspensions.iter().any(|s| &s.iklan_id == iklan_id);
            if suspended {
                results.push(SuspendResultItem {
                    iklan_id: *iklan_id,
                    success: true,
                    error: None,
                });
            } else {
                results.push(SuspendResultItem {
                    iklan_id: *iklan_id,
                    success: false,
                    error: Some("iklan tidak ditemukan atau sudah tersuspensi".into()),
                });
            }
        }

        // Kirim notifikasi ke tiap pemilik iklan yang berhasil di-suspend.
        if let (Some(notifier), Some(auth_client)) = (notifier, auth_client) {
            for s in &suspensions {
                let iklan = self.repo.find_by_id(s.iklan_id).await?;
                if let Some(iklan) = iklan {
                    let owner_id = iklan.poster_id;
                    let email = auth_client
                        .get_account_email(owner_id)
                        .await
                        .unwrap_or_default();
                    let title = if s.is_permanent {
                        "Iklan Anda telah ditangguhkan secara permanen"
                    } else {
                        "Iklan Anda telah ditangguhkan sementara"
                    };
                    let body = format!(
                        "Iklan \"{}\" (ID: {}) telah ditangguhkan karena: {}. \
                         Silakan hubungi admin untuk informasi lebih lanjut.",
                        iklan.judul, s.iklan_id, s.reason
                    );

                    let payload = notification_service_client::NotificationPayload {
                        title: title.to_string(),
                        body: body.clone(),
                        data: Some(serde_json::json!({
                            "type": "iklan_suspended",
                            "iklan_id": s.iklan_id.to_string(),
                            "is_permanent": s.is_permanent,
                        })),
                    };

                    // In-app notification (best-effort, jangan gagalkan suspend).
                    let _ = notifier.send(owner_id, payload).await;

                    // Email jika alamat tersedia.
                    if !email.is_empty() {
                        let _ = notifier
                            .send_email(notification_service_client::EmailMessage {
                                to: email,
                                subject: title.to_string(),
                                body,
                            })
                            .await;
                    }
                }
            }
        }

        Ok(SuspendResponse { results })
    }

    // ── Auto-expire temporary suspensions ────────────────────────────────────

    /// Dipanggil oleh scheduler periodik.
    /// Mengembalikan jumlah iklan yang di-un-suspend.
    pub async fn expire_temporary_suspensions(&self) -> Result<u64, anyhow::Error> {
        self.repo.expire_temporary_suspensions().await
    }
}

fn to_response(e: crate::domain::entity::IklanPekerjaan) -> IklanPekerjaanResponse {
    IklanPekerjaanResponse {
        id: e.id,
        poster_id: e.poster_id,
        judul: e.judul,
        perusahaan: e.perusahaan,
        deskripsi: e.deskripsi,
        lokasi: e.lokasi,
        gaji_min: e.gaji_min,
        gaji_max: e.gaji_max,
        tipe: e.tipe,
        foto_urls: e.foto_urls,
        is_active: e.is_active,
        moderation_status: e.moderation_status,
        created_at: e.created_at,
    }
}

fn to_admin_response(e: crate::domain::entity::IklanPekerjaan) -> AdminIklanPekerjaanResponse {
    AdminIklanPekerjaanResponse {
        id: e.id,
        poster_id: e.poster_id,
        judul: e.judul,
        perusahaan: e.perusahaan,
        deskripsi: e.deskripsi,
        lokasi: e.lokasi,
        gaji_min: e.gaji_min,
        gaji_max: e.gaji_max,
        tipe: e.tipe,
        foto_urls: e.foto_urls,
        is_active: e.is_active,
        moderation_status: e.moderation_status,
        deleted_at: e.deleted_at,
        created_at: e.created_at,
        updated_at: e.updated_at,
    }
}
