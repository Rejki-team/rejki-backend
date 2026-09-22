use std::sync::Arc;
use uuid::Uuid;

use super::dto::{
    AdminIklanPekerjaDetailResponse, AdminIklanPekerjaResponse, AdminListQuery,
    CreateIklanPekerjaInput, IklanPekerjaResponse, ListQuery, SuspendEvidenceInput, SuspendInput,
    SuspendResponse, SuspendResultItem, UpdatePekerjaInput,
};
use crate::domain::repository::{
    AdminListParams, CreatePekerjaParams, IklanPekerjaRepository, UpdatePekerjaParams,
};
use common_geocoding::{GeocodeInput, GeocodingClient};
use common_rate_limit::RateLimiter;
use notification_service_client::NotificationClient;
use region_service_client::RegionClient;
use storage_service_client::StorageClient;
use user_service_client::UserClient;

const DEFAULT_LIMIT: i64 = 20;
/// Radius default listing Iklan Pekerja (PRD §5.12.1) — bisa dioverride via `max_distance`
/// (mobile sudah mengirim ini, lihat `worker_remote_datasource.dart`).
const DEFAULT_RADIUS_KM: f64 = 2.0;
const CSV_MAX: i64 = 10_000;
/// Jenis dokumen sensitif yang bisa di-proxy-reveal via `UserClient` (F-27b).
const SENSITIVE_KINDS: [&str; 3] = ["nik", "ktp", "selfie"];
pub mod storage_category {
    pub const SUSPENSION_EVIDENCE: &str = "iklan-suspension-evidence";
}

pub struct IklanPekerjaService<R: IklanPekerjaRepository> {
    repo: Arc<R>,
    rate_limiter: Option<Arc<dyn RateLimiter>>,
    region_client: Option<Arc<dyn RegionClient>>,
    user_client: Option<Arc<dyn UserClient>>,
    geocoding_client: Option<Arc<dyn GeocodingClient>>,
}

impl<R: IklanPekerjaRepository> IklanPekerjaService<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self {
            repo,
            rate_limiter: None,
            region_client: None,
            user_client: None,
            geocoding_client: None,
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

    pub fn with_user_client(mut self, uc: Arc<dyn UserClient>) -> Self {
        self.user_client = Some(uc);
        self
    }

    pub fn with_geocoding_client(mut self, gc: Arc<dyn GeocodingClient>) -> Self {
        self.geocoding_client = Some(gc);
        self
    }

    /// Geocode `lokasi` (teks bebas) + nama wilayah dari `region_id` (F-1). `None` bila
    /// tidak ada geocoding client terpasang, tidak ada input untuk di-geocode, atau
    /// provider gagal — pemanggil menyimpan tanpa koordinat (degradasi anggun).
    async fn geocode_lokasi(
        &self,
        lokasi: Option<&str>,
        region_id: Option<&str>,
    ) -> Option<(f64, f64)> {
        let geocoding = self.geocoding_client.as_ref()?;
        if lokasi.is_none() && region_id.is_none() {
            return None;
        }
        let regency_name = match region_id {
            Some(rid) => self
                .region_client
                .as_ref()?
                .get_region(rid)
                .await
                .ok()
                .map(|r| r.name),
            None => None,
        };
        let input = GeocodeInput {
            address_line: lokasi.map(String::from),
            regency_name,
            ..Default::default()
        };
        match geocoding.geocode(&input).await {
            Ok(Some(c)) => Some((c.latitude, c.longitude)),
            Ok(None) => None,
            Err(e) => {
                tracing::warn!(error = ?e, "geocoding gagal — iklan disimpan tanpa koordinat");
                None
            }
        }
    }

    pub async fn list(&self, q: ListQuery) -> Result<Vec<IklanPekerjaResponse>, anyhow::Error> {
        // Filter radius (F-1, PRD §5.12.1): aktif hanya bila KEDUA lat/lng dikirim. `max_distance`
        // (mobile, km) meng-override radius default bila diisi.
        let radius = match (q.latitude, q.longitude) {
            (Some(lat), Some(lng)) => Some(common_geo::RadiusQuery {
                lat,
                lng,
                radius_km: q.max_distance.unwrap_or(DEFAULT_RADIUS_KM),
            }),
            _ => None,
        };
        Ok(self
            .repo
            .list(
                q.limit.unwrap_or(DEFAULT_LIMIT),
                q.offset.unwrap_or(0),
                radius,
            )
            .await?
            .into_iter()
            .map(to_response)
            .collect())
    }

    pub async fn get(&self, id: Uuid) -> Result<IklanPekerjaResponse, anyhow::Error> {
        self.repo
            .find_by_id(id)
            .await?
            .map(to_response)
            .ok_or_else(|| anyhow::anyhow!("tidak ditemukan"))
    }

    pub async fn create(
        &self,
        poster_id: Uuid,
        input: CreateIklanPekerjaInput,
    ) -> Result<IklanPekerjaResponse, anyhow::Error> {
        // Rate limit: 30 req/15 menit per user
        if let Some(rl) = &self.rate_limiter {
            if !rl
                .allow("iklan_pekerja:create", &poster_id.to_string())
                .await
            {
                return Err(anyhow::anyhow!(
                    "terlalu banyak permintaan, coba lagi nanti"
                ));
            }
        }
        if self.repo.is_poster_in_cooldown(poster_id).await? {
            return Err(anyhow::anyhow!(
                "Anda tidak dapat membuat iklan baru selama 3 hari setelah iklan ditangguhkan secara permanen"
            ));
        }
        let deskripsi = ammonia::clean_text(&input.deskripsi);
        let jam_kerja = input.jam_kerja.as_deref().map(ammonia::clean_text);
        let phone_number = input.phone_number.as_deref().map(ammonia::clean_text);
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

        let coords = self
            .geocode_lokasi(input.lokasi.as_deref(), input.region_id.as_deref())
            .await;

        Ok(to_response(
            self.repo
                .create(CreatePekerjaParams {
                    poster_id,
                    nama: &input.nama,
                    keahlian: &input.keahlian,
                    deskripsi: &deskripsi,
                    lokasi: input.lokasi.as_deref(),
                    region_id: input.region_id.as_deref(),
                    tarif_min: input.tarif_min,
                    tarif_max: input.tarif_max,
                    jam_kerja: jam_kerja.as_deref(),
                    phone_number: phone_number.as_deref(),
                    latitude: coords.map(|(lat, _)| lat),
                    longitude: coords.map(|(_, lng)| lng),
                })
                .await?,
        ))
    }

    pub async fn delete(&self, id: Uuid, poster_id: Uuid) -> Result<bool, anyhow::Error> {
        self.repo.delete(id, poster_id).await
    }

    pub async fn update(
        &self,
        poster_id: Uuid,
        id: Uuid,
        input: UpdatePekerjaInput,
    ) -> Result<IklanPekerjaResponse, anyhow::Error> {
        // Rate limit: 30 req/15 menit per user
        if let Some(rl) = &self.rate_limiter {
            if !rl
                .allow("iklan_pekerja:update", &poster_id.to_string())
                .await
            {
                return Err(anyhow::anyhow!(
                    "terlalu banyak permintaan, coba lagi nanti"
                ));
            }
        }

        let existing = self
            .repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("tidak ditemukan"))?;

        // Ownership check — return NotFound, not Forbidden
        if existing.poster_id != poster_id {
            return Err(anyhow::anyhow!("tidak ditemukan"));
        }

        // Lifecycle guard: only allow updates when moderation_status is Active
        if existing.moderation_status != crate::domain::entity::ModerationStatus::Active {
            return Err(anyhow::anyhow!(
                "iklan tidak dapat diubah dalam status moderasi saat ini"
            ));
        }

        // Sanitasi
        let nama = input.nama.map(|v| ammonia::clean_text(&v));
        let deskripsi = input.deskripsi.map(|v| ammonia::clean_text(&v));
        let jam_kerja = input.jam_kerja.map(|v| ammonia::clean_text(&v));
        let phone_number = input.phone_number.map(|v| ammonia::clean_text(&v));

        // Validasi region_id jika diisi.
        if let (Some(rc), Some(ref rid)) = (&self.region_client, &input.region_id) {
            if !rid.is_empty() {
                match rc.get_region(rid).await {
                    Err(region_service_client::RegionClientError::NotFound) => {
                        return Err(anyhow::anyhow!("region_id tidak ditemukan"));
                    }
                    Err(_) => {
                        tracing::warn!(region_id = %rid, "region-service unavailable saat validasi update");
                    }
                    Ok(_) => {}
                }
            }
        }

        // Re-geocode (F-1) hanya bila `lokasi`/`region_id` benar-benar diubah — hindari
        // panggilan geocoding sia-sia saat update lain (mis. ganti nama/tarif saja).
        let coords = if input.lokasi.is_some() || input.region_id.is_some() {
            self.geocode_lokasi(input.lokasi.as_deref(), input.region_id.as_deref())
                .await
        } else {
            None
        };

        Ok(to_response(
            self.repo
                .update(
                    id,
                    poster_id,
                    UpdatePekerjaParams {
                        nama,
                        keahlian: input.keahlian,
                        deskripsi,
                        lokasi: input.lokasi,
                        region_id: input.region_id,
                        tarif_min: input.tarif_min,
                        tarif_max: input.tarif_max,
                        jam_kerja,
                        phone_number,
                        foto_urls: input.foto_urls,
                        is_active: input.is_active,
                        latitude: coords.map(|(lat, _)| lat),
                        longitude: coords.map(|(_, lng)| lng),
                    },
                )
                .await?
                .ok_or_else(|| anyhow::anyhow!("tidak ditemukan"))?,
        ))
    }

    pub async fn admin_list(
        &self,
        query: AdminListQuery,
    ) -> Result<(Vec<AdminIklanPekerjaResponse>, i64), anyhow::Error> {
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

    /// Detail satu iklan untuk pop-up admin (F-27b) — termasuk indikator dokumen
    /// sensitif poster (NIK/KTP/Selfie), di-resolve via `UserClient` TANPA
    /// menyalin data sensitif ke schema `iklan-pekerja-service`. `Ok(None)` bila
    /// iklan tidak ditemukan (handler → 404).
    pub async fn admin_get_detail(
        &self,
        id: Uuid,
    ) -> Result<Option<AdminIklanPekerjaDetailResponse>, anyhow::Error> {
        let iklan = match self.repo.find_by_id(id).await? {
            Some(i) => i,
            None => return Ok(None),
        };

        let flags = match &self.user_client {
            Some(uc) => uc
                .get_sensitive_doc_flags(iklan.poster_id)
                .await
                .unwrap_or_else(|e| {
                    tracing::warn!(error = ?e, poster_id = %iklan.poster_id, "gagal resolve status dokumen sensitif — anggap tidak tersedia");
                    Default::default()
                }),
            None => Default::default(),
        };

        Ok(Some(AdminIklanPekerjaDetailResponse {
            id: iklan.id,
            poster_id: iklan.poster_id,
            nama: iklan.nama,
            keahlian: iklan.keahlian,
            deskripsi: iklan.deskripsi,
            lokasi: iklan.lokasi,
            region_id: iklan.region_id,
            tarif_min: iklan.tarif_min,
            tarif_max: iklan.tarif_max,
            jam_kerja: iklan.jam_kerja,
            phone_number: iklan.phone_number,
            foto_urls: iklan.foto_urls,
            is_active: iklan.is_active,
            moderation_status: iklan.moderation_status,
            deleted_at: iklan.deleted_at,
            created_at: iklan.created_at,
            updated_at: iklan.updated_at,
            has_nik: flags.has_nik,
            has_ktp: flags.has_ktp,
            has_selfie: flags.has_selfie,
        }))
    }

    /// Proxy reveal data sensitif (`kind` ∈ nik/ktp/selfie) milik poster suatu
    /// iklan, untuk admin (F-27b). Audit TETAP tercatat tunggal di user-service —
    /// `iklan-pekerja-service` tidak menyimpan/mencatat ulang apa pun di sini.
    /// `Ok(None)` bila iklan tidak ditemukan ATAU data belum tersedia (handler → 404).
    pub async fn admin_reveal_sensitive(
        &self,
        id: Uuid,
        kind: &str,
        admin_id: Uuid,
    ) -> Result<Option<String>, anyhow::Error> {
        if !SENSITIVE_KINDS.contains(&kind) {
            return Err(anyhow::anyhow!(
                "jenis data tidak dikenal: {kind} (gunakan nik, ktp, atau selfie)"
            ));
        }
        let Some(uc) = &self.user_client else {
            return Err(anyhow::anyhow!("user-service client tidak tersedia"));
        };
        let iklan = match self.repo.find_by_id(id).await? {
            Some(i) => i,
            None => return Ok(None),
        };

        let result = if kind == "nik" {
            uc.admin_reveal_nik(iklan.poster_id, admin_id).await
        } else {
            uc.admin_get_document_url(iklan.poster_id, kind, admin_id)
                .await
        };
        result.map_err(|e| anyhow::anyhow!("gagal mengambil data sensitif: {e}"))
    }

    pub async fn admin_export_csv(
        &self,
        query: AdminListQuery,
    ) -> Result<Vec<AdminIklanPekerjaResponse>, anyhow::Error> {
        let params = AdminListParams {
            q: query.q,
            moderation_status: query.status,
            sort_by: query.sort_by,
            sort_dir: query.sort_dir,
            limit: CSV_MAX,
            offset: 0,
        };
        let items = self.repo.admin_list_all(params).await?;
        Ok(items.into_iter().map(to_admin_response).collect())
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
                        iklan.nama, s.iklan_id, s.reason
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

fn to_response(e: crate::domain::entity::IklanPekerja) -> IklanPekerjaResponse {
    IklanPekerjaResponse {
        id: e.id,
        poster_id: e.poster_id,
        nama: e.nama,
        keahlian: e.keahlian,
        deskripsi: e.deskripsi,
        lokasi: e.lokasi,
        region_id: e.region_id,
        tarif_min: e.tarif_min,
        tarif_max: e.tarif_max,
        jam_kerja: e.jam_kerja,
        phone_number: e.phone_number,
        foto_urls: e.foto_urls,
        is_active: e.is_active,
        moderation_status: e.moderation_status,
        created_at: e.created_at,
    }
}

fn to_admin_response(e: crate::domain::entity::IklanPekerja) -> AdminIklanPekerjaResponse {
    AdminIklanPekerjaResponse {
        id: e.id,
        poster_id: e.poster_id,
        nama: e.nama,
        keahlian: e.keahlian,
        deskripsi: e.deskripsi,
        lokasi: e.lokasi,
        region_id: e.region_id,
        tarif_min: e.tarif_min,
        tarif_max: e.tarif_max,
        jam_kerja: e.jam_kerja,
        phone_number: e.phone_number,
        foto_urls: e.foto_urls,
        is_active: e.is_active,
        moderation_status: e.moderation_status,
        deleted_at: e.deleted_at,
        created_at: e.created_at,
        updated_at: e.updated_at,
    }
}
