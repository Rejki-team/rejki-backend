use std::sync::Arc;
use uuid::Uuid;

use std::collections::HashMap;

use super::dto::{
    AdminIklanBarangBekasResponse, AdminListQuery, BiderResponse, BiderWithIklanResponse,
    CreateIklanBarangBekasInput, IklanBarangBekasResponse, ListQuery, SetujuiBiderInput,
    SuspendEvidenceInput, SuspendInput, SuspendResponse, SuspendResultItem, UpdateBarangBekasInput,
};
use crate::domain::entity::{AvailabilityStatus, Bider, IklanBarangBekas, ModerationStatus};
use crate::domain::repository::{
    AdminListParams, CreateBarangBekasParams, IklanBarangBekasRepository, UpdateBarangBekasParams,
};
use chat_service_client::ChatClient;
use common_geocoding::{GeocodeInput, GeocodingClient};
use common_rate_limit::RateLimiter;
use notification_service_client::NotificationClient;
use region_service_client::RegionClient;
use storage_service_client::StorageClient;
use user_service_client::UserClient;

const DEFAULT_LIMIT: i64 = 20;
const CSV_MAX: i64 = 10_000;
/// Radius default listing Iklan Barang Bekas (PRD §5.14.1).
const DEFAULT_RADIUS_KM: f64 = 10.0;
/// Jenis barang yang sah — digunakan validasi service-side (defense-in-depth).
const VALID_JENIS_BARANG: [&str; 2] = ["bekas", "baru"];
pub mod storage_category {
    pub const SUSPENSION_EVIDENCE: &str = "iklan-suspension-evidence";
}

pub struct IklanBarangBekasService<R: IklanBarangBekasRepository> {
    repo: Arc<R>,
    rate_limiter: Option<Arc<dyn RateLimiter>>,
    region_client: Option<Arc<dyn RegionClient>>,
    geocoding_client: Option<Arc<dyn GeocodingClient>>,
    /// Enrichment "daftar bider" (F-15, Kelompok 3 Phase 3) — nama + lokasi peminat.
    user_client: Option<Arc<dyn UserClient>>,
    /// F-19 (Kelompok 4 Phase 4): auto-end Chat setelah barang disetujui/diambil.
    chat_client: Option<Arc<dyn ChatClient>>,
}

impl<R: IklanBarangBekasRepository> IklanBarangBekasService<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self {
            repo,
            rate_limiter: None,
            region_client: None,
            geocoding_client: None,
            user_client: None,
            chat_client: None,
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

    pub fn with_geocoding_client(mut self, gc: Arc<dyn GeocodingClient>) -> Self {
        self.geocoding_client = Some(gc);
        self
    }

    pub fn with_user_client(mut self, uc: Arc<dyn UserClient>) -> Self {
        self.user_client = Some(uc);
        self
    }

    pub fn with_chat_client(mut self, cc: Arc<dyn ChatClient>) -> Self {
        self.chat_client = Some(cc);
        self
    }

    /// Geocode `lokasi` (teks bebas) + nama wilayah dari `region_id` (F-1). `None` bila
    /// tidak ada geocoding client terpasang, tidak ada input, atau provider gagal —
    /// pemanggil menyimpan tanpa koordinat (degradasi anggun).
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

    pub async fn list(&self, q: ListQuery) -> Result<Vec<IklanBarangBekasResponse>, anyhow::Error> {
        // Filter radius (F-1, PRD §5.14.1): aktif hanya bila KEDUA lat/lng dikirim.
        let radius = match (q.latitude, q.longitude) {
            (Some(lat), Some(lng)) => Some(common_geo::RadiusQuery {
                lat,
                lng,
                radius_km: DEFAULT_RADIUS_KM,
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
            .map(to_resp)
            .collect())
    }

    /// "Iklan Saya" (P4.10) — daftar iklan milik seller yang login, entry point ke
    /// "Kelola Iklan Saya" (PRD §5.14.2). Tidak difilter availability/moderasi.
    pub async fn list_my_ads(
        &self,
        seller_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<IklanBarangBekasResponse>, anyhow::Error> {
        Ok(self
            .repo
            .list_by_seller(seller_id, limit, offset)
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

        let coords = self
            .geocode_lokasi(lokasi.as_deref(), input.region_id.as_deref())
            .await;

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
                    latitude: coords.map(|(lat, _)| lat),
                    longitude: coords.map(|(_, lng)| lng),
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

    pub async fn update(
        &self,
        user_id: Uuid,
        id: Uuid,
        input: UpdateBarangBekasInput,
    ) -> Result<IklanBarangBekasResponse, anyhow::Error> {
        // 1. Rate limit: 30 req/15 menit per user
        if let Some(rl) = &self.rate_limiter {
            if !rl
                .allow("iklan_barang_bekas:update", &user_id.to_string())
                .await
            {
                return Err(anyhow::anyhow!("terlalu banyak permintaan"));
            }
        }

        // 2. Find existing to check lifecycle
        let existing = self
            .repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("tidak ditemukan"))?;

        // 3. Ownership check (IDOR → 404)
        if existing.seller_id != user_id {
            return Err(anyhow::anyhow!("tidak ditemukan"));
        }

        // 4. Lifecycle guard
        if existing.moderation_status != ModerationStatus::Active {
            return Err(anyhow::anyhow!("iklan sedang ditangguhkan"));
        }
        if existing.availability_status != AvailabilityStatus::Tersedia {
            return Err(anyhow::anyhow!("iklan sudah tidak tersedia"));
        }

        // 5. Sanitasi
        let judul = input.judul.map(|s| ammonia::clean_text(&s));
        let deskripsi = input.deskripsi.map(|s| ammonia::clean_text(&s));
        let lokasi_pengambilan = input.lokasi_pengambilan.map(|s| ammonia::clean_text(&s));
        let lokasi = input.lokasi.map(|s| ammonia::clean_text(&s));

        // 6. Validasi region_id
        if let (Some(rc), Some(ref rid)) = (&self.region_client, &input.region_id) {
            if !rid.is_empty() {
                match rc.get_region(rid).await {
                    Err(region_service_client::RegionClientError::NotFound) => {
                        return Err(anyhow::anyhow!("region_id tidak valid"));
                    }
                    Err(_) => {
                        tracing::warn!(region_id = %rid, "region-service unavailable saat validasi update");
                    }
                    Ok(_) => {}
                }
            }
        }

        // Re-geocode (F-1) hanya bila `lokasi`/`region_id` benar-benar diubah.
        let coords = if lokasi.is_some() || input.region_id.is_some() {
            self.geocode_lokasi(lokasi.as_deref(), input.region_id.as_deref())
                .await
        } else {
            None
        };

        // 7. Build params & update
        let params = UpdateBarangBekasParams {
            judul,
            deskripsi,
            jenis_barang: input.jenis_barang,
            jumlah: input.jumlah,
            lokasi_pengambilan,
            lokasi,
            region_id: input.region_id,
            foto_urls: input.foto_urls,
            is_active: input.is_active,
            latitude: coords.map(|(lat, _)| lat),
            longitude: coords.map(|(_, lng)| lng),
        };

        self.repo
            .update(id, user_id, params)
            .await?
            .map(to_resp)
            .ok_or_else(|| anyhow::anyhow!("tidak ditemukan"))
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

    // ── Bider (F-15, Kelompok 3 Phase 3, PRD §5.14.1-5.14.2 Gambar 5) ────────────

    /// P3.2: "Menekan Ambil Barang" → jadi bider. Validasi: bukan iklan sendiri, iklan masih
    /// tersedia & tidak ditangguhkan, belum ada bid aktif (menunggu) milik peminat yang sama.
    pub async fn ambil(
        &self,
        peminat_id: Uuid,
        iklan_id: Uuid,
    ) -> Result<BiderResponse, anyhow::Error> {
        let iklan = self
            .repo
            .find_by_id(iklan_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("tidak ditemukan"))?;
        if iklan.seller_id == peminat_id {
            return Err(anyhow::anyhow!(
                "tidak dapat mengambil barang milik sendiri"
            ));
        }
        if iklan.moderation_status != ModerationStatus::Active {
            return Err(anyhow::anyhow!("iklan sedang ditangguhkan"));
        }
        if iklan.availability_status != AvailabilityStatus::Tersedia {
            return Err(anyhow::anyhow!("barang sudah tidak tersedia"));
        }
        if self.repo.has_pending_bider(iklan_id, peminat_id).await? {
            return Err(anyhow::anyhow!(
                "Anda sudah mengajukan pengambilan untuk barang ini"
            ));
        }
        let bider = self.repo.create_bider(iklan_id, peminat_id).await?;
        Ok(to_bider_response_basic(bider))
    }

    /// P3.3: "Kelola Iklan Saya" → daftar bider (nama, jarak, status kontak). Ownership check
    /// via `find_by_id` (pola kembar `list_lamaran_for_iklan`, IDOR→404 di layer application).
    /// Enrichment batched (Hazard #5): satu panggilan `UserClient` untuk semua peminat, dedup
    /// resolusi nama kelurahan/kecamatan per region id UNIK (bukan per bider).
    pub async fn list_bider(
        &self,
        owner_id: Uuid,
        iklan_id: Uuid,
    ) -> Result<Vec<BiderResponse>, anyhow::Error> {
        let iklan = self
            .repo
            .find_by_id(iklan_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("tidak ditemukan"))?;
        if iklan.seller_id != owner_id {
            return Err(anyhow::anyhow!("tidak ditemukan"));
        }
        let items = self.repo.list_bider_for_iklan(iklan_id).await?;
        if items.is_empty() {
            return Ok(Vec::new());
        }

        let peminat_ids: Vec<Uuid> = items.iter().map(|b| b.peminat_id).collect();
        let locations: HashMap<Uuid, user_service_client::UserLocationSummary> =
            match &self.user_client {
                Some(client) => client
                    .get_location_summaries_by_auth_ids(&peminat_ids)
                    .await
                    .map(|v| v.into_iter().map(|s| (s.auth_id, s)).collect())
                    .unwrap_or_default(),
                None => Default::default(),
            };

        // Resolusi nama wilayah — dedup per region id UNIK dulu (Hazard #5: hindari 1 lookup
        // per bider bila banyak bider berbagi kelurahan/kecamatan yang sama).
        let mut region_names: HashMap<String, String> = HashMap::new();
        if let Some(rc) = &self.region_client {
            let mut unique_ids: Vec<String> = Vec::new();
            for loc in locations.values() {
                for id in [&loc.village_id, &loc.district_id].into_iter().flatten() {
                    if !unique_ids.contains(id) {
                        unique_ids.push(id.clone());
                    }
                }
            }
            for id in unique_ids {
                if let Ok(r) = rc.get_region(&id).await {
                    region_names.insert(id, r.name);
                }
            }
        }

        Ok(items
            .into_iter()
            .map(|b| {
                let loc = locations.get(&b.peminat_id);
                let jarak_km = match (
                    loc.and_then(|l| l.latitude),
                    loc.and_then(|l| l.longitude),
                    iklan.latitude,
                    iklan.longitude,
                ) {
                    (Some(blat), Some(blng), Some(ilat), Some(ilng)) => {
                        Some(common_geo::haversine_km(ilat, ilng, blat, blng))
                    }
                    _ => None,
                };
                BiderResponse {
                    id: b.id,
                    iklan_id: b.iklan_id,
                    peminat_id: b.peminat_id,
                    peminat_nama: loc.map(|l| l.username.clone()),
                    kelurahan: loc
                        .and_then(|l| l.village_id.as_ref())
                        .and_then(|v| region_names.get(v).cloned()),
                    kecamatan: loc
                        .and_then(|l| l.district_id.as_ref())
                        .and_then(|d| region_names.get(d).cloned()),
                    jarak_km,
                    status: b.status,
                    sudah_menghubungi: b.sudah_menghubungi,
                    created_at: b.created_at,
                }
            })
            .collect())
    }

    /// P3.4: "Tombol Setujui Bider" — Bider→Disetujui, Iklan→SudahDiambil, bider lain
    /// ditandai Withdrawn. Ownership check di repository (IDOR→404).
    /// F-19 (Kelompok 4 Phase 4): "proses pada iklan terkait selesai" (barang sudah
    /// diambil) — memicu auto-end Chat 2x24 jam untuk percakapan terkait iklan ini.
    pub async fn setujui_bider(
        &self,
        owner_id: Uuid,
        bider_id: Uuid,
        input: SetujuiBiderInput,
    ) -> Result<BiderResponse, anyhow::Error> {
        let bider = self
            .repo
            .setujui_bider(bider_id, owner_id, input.sudah_menghubungi)
            .await?
            .ok_or_else(|| anyhow::anyhow!("bider tidak ditemukan"))?;

        // Fail-open (§4.5 backend I/O aman) — kegagalan menjadwalkan auto-end Chat
        // TIDAK boleh menggagalkan operasi utama (persetujuan bider sudah tersimpan).
        if let Some(chat_client) = &self.chat_client {
            if let Err(e) = chat_client
                .schedule_auto_end_for_ad("barang_bekas", bider.iklan_id)
                .await
            {
                tracing::warn!(
                    error = ?e,
                    iklan_id = %bider.iklan_id,
                    "gagal menjadwalkan auto-end chat — dilewati (fail-open)"
                );
            }
        }

        Ok(to_bider_response_basic(bider))
    }

    /// P3.5: "Tombol Withdraw Bider" — Bider→Withdrawn; bila sebelumnya Disetujui, iklan
    /// otomatis re-listing (kembali Tersedia). Ownership check di repository (IDOR→404).
    pub async fn withdraw_bider(
        &self,
        owner_id: Uuid,
        bider_id: Uuid,
    ) -> Result<BiderResponse, anyhow::Error> {
        self.repo
            .withdraw_bider(bider_id, owner_id)
            .await?
            .map(to_bider_response_basic)
            .ok_or_else(|| anyhow::anyhow!("bider tidak ditemukan"))
    }

    /// "Bider Saya" (P4.11, Riwayat → Aktifitas → Barang Bekas) — daftar bid milik
    /// peminat yang login + konteks iklan (batched, Hazard #5).
    pub async fn list_bider_saya(
        &self,
        peminat_id: Uuid,
    ) -> Result<Vec<BiderWithIklanResponse>, anyhow::Error> {
        let items = self.repo.list_bider_for_peminat(peminat_id).await?;
        if items.is_empty() {
            return Ok(Vec::new());
        }

        let iklan_ids: Vec<Uuid> = items.iter().map(|b| b.iklan_id).collect();
        let iklan_by_id: HashMap<Uuid, IklanBarangBekas> = self
            .repo
            .find_by_ids(&iklan_ids)
            .await?
            .into_iter()
            .map(|i| (i.id, i))
            .collect();

        Ok(items
            .into_iter()
            .map(|b| {
                let iklan = iklan_by_id.get(&b.iklan_id);
                BiderWithIklanResponse {
                    id: b.id,
                    iklan_id: b.iklan_id,
                    peminat_id: b.peminat_id,
                    status: b.status,
                    sudah_menghubungi: b.sudah_menghubungi,
                    created_at: b.created_at,
                    iklan_judul: iklan.map(|i| i.judul.clone()),
                    iklan_deskripsi: iklan.map(|i| i.deskripsi.clone()),
                    iklan_jenis_barang: iklan.map(|i| i.jenis_barang.clone()),
                    iklan_jumlah: iklan.map(|i| i.jumlah),
                    iklan_lokasi_pengambilan: iklan.map(|i| i.lokasi_pengambilan.clone()),
                    iklan_foto_urls: iklan.map(|i| i.foto_urls.clone()).unwrap_or_default(),
                    iklan_availability_status: iklan.map(|i| i.availability_status),
                }
            })
            .collect())
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

/// Response bider tanpa enrichment nama/lokasi/jarak — dipakai untuk `ambil`/`setujui_bider`/
/// `withdraw_bider`, di mana pemanggil sudah tahu identitasnya sendiri (tidak perlu di-lookup).
fn to_bider_response_basic(b: Bider) -> BiderResponse {
    BiderResponse {
        id: b.id,
        iklan_id: b.iklan_id,
        peminat_id: b.peminat_id,
        peminat_nama: None,
        kelurahan: None,
        kecamatan: None,
        jarak_km: None,
        status: b.status,
        sudah_menghubungi: b.sudah_menghubungi,
        created_at: b.created_at,
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
