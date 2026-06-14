use std::sync::Arc;
use uuid::Uuid;

use super::dto::{
    AdminIklanBarangBekasResponse, AdminListQuery, CreateIklanBarangBekasInput,
    IklanBarangBekasResponse, ListQuery, SuspendEvidenceInput, SuspendInput, SuspendResponse,
    SuspendResultItem,
};
use crate::domain::repository::{
    AdminListParams, CreateBarangBekasParams, IklanBarangBekasRepository,
};
use notification_service_client::NotificationClient;
use storage_service_client::StorageClient;

const DEFAULT_LIMIT: i64 = 20;
const CSV_MAX: i64 = 10_000;
pub mod storage_category {
    pub const SUSPENSION_EVIDENCE: &str = "iklan-suspension-evidence";
}

pub struct IklanBarangBekasService<R: IklanBarangBekasRepository> {
    repo: Arc<R>,
}

impl<R: IklanBarangBekasRepository> IklanBarangBekasService<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
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
        if self.repo.is_poster_in_cooldown(seller_id).await? {
            return Err(anyhow::anyhow!(
                "Anda tidak dapat membuat iklan baru selama 3 hari setelah iklan ditangguhkan secara permanen"
            ));
        }
        let judul = ammonia::clean_text(&input.judul);
        let deskripsi = ammonia::clean_text(&input.deskripsi);
        let foto_urls = input.foto_urls.unwrap_or_default();
        Ok(to_resp(
            self.repo
                .create(CreateBarangBekasParams {
                    seller_id,
                    judul: &judul,
                    deskripsi: &deskripsi,
                    harga: input.harga,
                    kondisi: &input.kondisi,
                    lokasi: input.lokasi.as_deref(),
                    foto_urls: &foto_urls,
                })
                .await?,
        ))
    }

    pub async fn mark_sold(&self, id: Uuid, seller_id: Uuid) -> Result<bool, anyhow::Error> {
        self.repo.mark_sold(id, seller_id).await
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

fn to_resp(e: crate::domain::entity::IklanBarangBekas) -> IklanBarangBekasResponse {
    IklanBarangBekasResponse {
        id: e.id,
        seller_id: e.seller_id,
        judul: e.judul,
        deskripsi: e.deskripsi,
        harga: e.harga,
        kondisi: e.kondisi,
        lokasi: e.lokasi,
        foto_urls: e.foto_urls,
        is_sold: e.is_sold,
        moderation_status: e.moderation_status,
        created_at: e.created_at,
    }
}

fn to_admin_resp(e: crate::domain::entity::IklanBarangBekas) -> AdminIklanBarangBekasResponse {
    AdminIklanBarangBekasResponse {
        id: e.id,
        seller_id: e.seller_id,
        judul: e.judul,
        deskripsi: e.deskripsi,
        harga: e.harga,
        kondisi: e.kondisi,
        lokasi: e.lokasi,
        foto_urls: e.foto_urls,
        is_sold: e.is_sold,
        moderation_status: e.moderation_status,
        deleted_at: e.deleted_at,
        created_at: e.created_at,
        updated_at: e.updated_at,
    }
}
