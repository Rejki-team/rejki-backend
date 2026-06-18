use std::sync::Arc;
use uuid::Uuid;

use super::dto::{
    AdminIklanBarangBekasResponse, AdminListQuery, CreateIklanBarangBekasInput,
    IklanBarangBekasResponse, ListQuery, SuspendEvidenceInput, SuspendInput, SuspendResponse,
    SuspendResultItem,
};
use crate::domain::entity::IklanBarangBekas;
use crate::domain::repository::{
    AdminListParams, CreateBarangBekasParams, IklanBarangBekasRepository,
};
use common_rate_limit::RateLimiter;
use notification_service_client::NotificationClient;
use region_service_client::RegionClient;
use storage_service_client::StorageClient;

const DEFAULT_LIMIT: i64 = 20;
const CSV_MAX: i64 = 10_000;
/// Jenis barang yang sah — digunakan validasi service-side (defense-in-depth).
const VALID_JENIS_BARANG: [&str; 2] = ["bekas", "baru"];
pub mod storage_category {
    pub const SUSPENSION_EVIDENCE: &str = "iklan-suspension-evidence";
}

pub struct IklanBarangBekasService<R: IklanBarangBekasRepository> {
    repo: Arc<R>,
    rate_limiter: Option<Arc<dyn RateLimiter>>,
    region_client: Option<Arc<dyn RegionClient>>,
}

impl<R: IklanBarangBekasRepository> IklanBarangBekasService<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self {
            repo,
            rate_limiter: None,
            region_client: None,
        }
    }
    pub fn with_rate_limiter(mut self, rl: Arc<dyn RateLimiter>) -> Self {
        self.rate_limiter = Some(rl);
        self
    }

    pub fn with_region_client(mut self, rc: Arc<dyn RegionClient>) -> Self {
        self.region_client = Some(rc);
        self
    }

    pub async fn list(&self, q: ListQuery) -> Result<Vec<IklanBarangBekasResponse>, anyhow::Error> {
        Ok(self
            .repo
            .list(q.limit.unwrap_or(DEFAULT_LIMIT), q.offset.unwrap_or(0))
            .await?
            .into_iter()
            .map(to_resp)
            .collect())
    }

    pub async fn get(&self, id: Uuid) -> Result<IklanBarangBekasResponse, anyhow::Error> {
        self.repo
            .find_by_id(id)
            .await?
            .map(to_resp)
            .ok_or_else(|| anyhow::anyhow!("tidak ditemukan"))
    }

    pub async fn create(
        &self,
        seller_id: Uuid,
        input: CreateIklanBarangBekasInput,
    ) -> Result<IklanBarangBekasResponse, anyhow::Error> {
        // Rate limit: 30 req/15 menit per seller
        if let Some(rl) = &self.rate_limiter {
            if !rl
                .allow("iklan_barang_bekas:create", &seller_id.to_string())
                .await
            {
                return Err(anyhow::anyhow!(
                    "terlalu banyak permintaan, coba lagi nanti"
                ));
            }
        }
        if self.repo.is_poster_in_cooldown(seller_id).await? {
            return Err(anyhow::anyhow!(
                "Anda tidak dapat membuat iklan baru selama 3 hari setelah iklan ditangguhkan secara permanen"
            ));
        }

        // Validasi jenis_barang — defense-in-depth (DB CHECK + service).
        if !VALID_JENIS_BARANG.contains(&input.jenis_barang.as_str()) {
            return Err(anyhow::anyhow!(
                "jenis_barang harus 'bekas' atau 'baru', bukan: {}",
                input.jenis_barang
            ));
        }

        let judul = ammonia::clean_text(&input.judul);
        let deskripsi = ammonia::clean_text(&input.deskripsi);
        let lokasi_pengambilan = ammonia::clean_text(&input.lokasi_pengambilan);
        let lokasi = input.lokasi.as_deref().map(ammonia::clean_text);

        // Validasi region_id jika diisi.
        if let (Some(rc), Some(ref rid)) = (&self.region_client, &input.region_id) {
            if !rid.is_empty() {
                match rc.get_region(rid).await {
                    Err(region_service_client::RegionClientError::NotFound) => {
                        return Err(anyhow::anyhow!("region_id tidak ditemukan"));
                    }
                    Err(_) => {
                        tracing::warn!(region_id = %rid, "region-service unavailable saat validasi create");
                    }
                    Ok(_) => {}
                }
            }
        }

        let foto_urls = input.foto_urls.unwrap_or_default();
        Ok(to_resp(
            self.repo
                .create(CreateBarangBekasParams {
                    seller_id,
                    judul: &judul,
                    deskripsi: &deskripsi,
                    jenis_barang: &input.jenis_barang,
                    jumlah: input.jumlah,
                    lokasi_pengambilan: &lokasi_pengambilan,
                    lokasi: lokasi.as_deref(),
                    region_id: input.region_id.as_deref(),
                    foto_urls: &foto_urls,
                })
                .await?,
        ))
    }

    /// Tandai barang sebagai "sudah diambil". Hanya pemilik (seller_id).
    /// Idempoten: bila sudah "sudah_diambil", UPDATE tidak berpengaruh → false.
    pub async fn mark_taken(&self, id: Uuid, seller_id: Uuid) -> Result<bool, anyhow::Error> {
        self.repo.mark_taken(id, seller_id).await
    }

    pub async fn delete(&self, id: Uuid, seller_id: Uuid) -> Result<bool, anyhow::Error> {
        self.repo.delete(id, seller_id).await
    }

    pub async fn admin_list(
        &self,
        query: AdminListQuery,
    ) -> Result<(Vec<AdminIklanBarangBekasResponse>, i64), anyhow::Error> {
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
            result.items.into_iter().map(to_admin_resp).collect(),
            result.total,
        ))
    }

    pub async fn admin_export_csv(
        &self,
        query: AdminListQuery,
    ) -> Result<Vec<AdminIklanBarangBekasResponse>, anyhow::Error> {
        let params = AdminListParams {
            q: query.q,
            moderation_status: query.status,
            sort_by: query.sort_by,
            sort_dir: query.sort_dir,
            limit: CSV_MAX,
            offset: 0,
        };
        let items = self.repo.admin_list_all(params).await?;
        Ok(items.into_iter().map(to_admin_resp).collect())
    }

    pub async fn request_suspend_evidence(
        &self,
        storage: &dyn StorageClient,
        admin_id: Uuid,
        input: SuspendEvidenceInput,
    ) -> Result<storage_service_client::UploadPermission, anyhow::Error> {
        storage
            .request_upload(
                storage_category::SUSPENSION_EVIDENCE,
                admin_id,
                storage_service_client::FileInfo {
                    mime: input.mime,
                    size_bytes: input.size_bytes,
                },
            )
            .await
            .map_err(|e| anyhow::anyhow!("storage error: {e}"))
    }

    pub async fn suspend(
        &self,
        admin_id: Uuid,
        input: SuspendInput,
        notifier: Option<&dyn NotificationClient>,
        auth_client: Option<&dyn auth_service_client::AuthClient>,
    ) -> Result<SuspendResponse, anyhow::Error> {
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

        if let (Some(notifier), Some(auth_client)) = (notifier, auth_client) {
            for s in &suspensions {
                let iklan = self.repo.find_by_id(s.iklan_id).await?;
                if let Some(iklan) = iklan {
                    let owner_id = iklan.seller_id;
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
                    let _ = notifier.send(owner_id, payload).await;
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

    pub async fn expire_temporary_suspensions(&self) -> Result<u64, anyhow::Error> {
        self.repo.expire_temporary_suspensions().await
    }
}

// ── Mapping helpers — entity → DTO (tanpa hardcode field) ────────────────────

fn to_resp(e: IklanBarangBekas) -> IklanBarangBekasResponse {
    IklanBarangBekasResponse {
        id: e.id,
        seller_id: e.seller_id,
        judul: e.judul,
        deskripsi: e.deskripsi,
        jenis_barang: e.jenis_barang,
        jumlah: e.jumlah,
        lokasi_pengambilan: e.lokasi_pengambilan,
        lokasi: e.lokasi,
        region_id: e.region_id,
        foto_urls: e.foto_urls,
        availability_status: e.availability_status,
        moderation_status: e.moderation_status,
        created_at: e.created_at,
    }
}

fn to_admin_resp(e: IklanBarangBekas) -> AdminIklanBarangBekasResponse {
    AdminIklanBarangBekasResponse {
        id: e.id,
        seller_id: e.seller_id,
        judul: e.judul,
        deskripsi: e.deskripsi,
        jenis_barang: e.jenis_barang,
        jumlah: e.jumlah,
        lokasi_pengambilan: e.lokasi_pengambilan,
        lokasi: e.lokasi,
        region_id: e.region_id,
        foto_urls: e.foto_urls,
        availability_status: e.availability_status,
        moderation_status: e.moderation_status,
        deleted_at: e.deleted_at,
        created_at: e.created_at,
        updated_at: e.updated_at,
    }
}
