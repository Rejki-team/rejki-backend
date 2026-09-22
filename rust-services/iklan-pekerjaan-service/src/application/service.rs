use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use super::dto::{
    AdminIklanPekerjaanResponse, AdminListQuery, BatalkanLamaranInput, CreateIklanPekerjaanInput,
    IklanPekerjaanResponse, LamarInput, LamaranResponse, LamaranWithIklanResponse,
    LamaranWithPelamarResponse, ListQuery, MulaiBekerjaInput, ReviewLamaranInput,
    SuspendEvidenceInput, SuspendInput, SuspendResponse, SuspendResultItem, UpdatePekerjaanInput,
};
use crate::domain::entity::{Lamaran, ModerationStatus};
use crate::domain::repository::{
    AdminListParams, CreateLamaranParams, CreatePekerjaanParams, IklanPekerjaanRepository,
    UpdatePekerjaanParams,
};
use chat_service_client::ChatClient;
use common_geocoding::{GeocodeInput, GeocodingClient};
use common_rate_limit::RateLimiter;
use iklan_pekerja_service_client::IklanPekerjaClient;
use notification_service_client::NotificationClient;
use region_service_client::RegionClient;
use storage_service_client::StorageClient;

/// Default untuk pagination & storage.
const DEFAULT_LIMIT: i64 = 20;
const CSV_MAX: i64 = 10_000;
/// Radius default listing Iklan Pekerjaan (PRD §5.11.1) — tidak ada override dari mobile
/// (`job_query_params.dart` hanya kirim `latitude`/`longitude`, tanpa parameter jarak).
const DEFAULT_RADIUS_KM: f64 = 2.0;
/// Geofence "Mulai Bekerja" (F-3, PRD §5.11.4) — pekerja harus berada dalam 50m dari
/// lokasi iklan. Helper jarak sama dengan radius listing (Kelompok 2 Phase 3.4).
const GEOFENCE_RADIUS_KM: f64 = 0.05;
/// Batas waktu pembatalan lamaran diterima oleh pemilik iklan (P1.7, PRD §5.11.5).
const CANCELLATION_CUTOFF_HOURS: i64 = 24;
pub mod storage_category {
    pub const SUSPENSION_EVIDENCE: &str = "iklan-suspension-evidence";
}

pub struct IklanPekerjaanService<R: IklanPekerjaanRepository> {
    repo: Arc<R>,
    rate_limiter: Option<Arc<dyn RateLimiter>>,
    region_client: Option<Arc<dyn RegionClient>>,
    geocoding_client: Option<Arc<dyn GeocodingClient>>,
    iklan_pekerja_client: Option<Arc<dyn IklanPekerjaClient>>,
    chat_client: Option<Arc<dyn ChatClient>>,
}

impl<R: IklanPekerjaanRepository> IklanPekerjaanService<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self {
            repo,
            rate_limiter: None,
            region_client: None,
            geocoding_client: None,
            iklan_pekerja_client: None,
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

    pub fn with_iklan_pekerja_client(mut self, c: Arc<dyn IklanPekerjaClient>) -> Self {
        self.iklan_pekerja_client = Some(c);
        self
    }

    /// F-19 (Kelompok 4 Phase 4): dipakai `tandai_selesai` untuk memicu auto-end
    /// percakapan Chat terkait iklan ini (2x24 jam).
    pub fn with_chat_client(mut self, c: Arc<dyn ChatClient>) -> Self {
        self.chat_client = Some(c);
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

    // ── Public endpoints ──────────────────────────────────────────────────────

    pub async fn list(
        &self,
        query: ListQuery,
    ) -> Result<Vec<IklanPekerjaanResponse>, anyhow::Error> {
        // Filter radius (F-1, PRD §5.11.1): aktif hanya bila KEDUA lat/lng dikirim.
        let radius = match (query.latitude, query.longitude) {
            (Some(lat), Some(lng)) => Some(common_geo::RadiusQuery {
                lat,
                lng,
                radius_km: DEFAULT_RADIUS_KM,
            }),
            _ => None,
        };
        let items = self
            .repo
            .list(
                query.limit.unwrap_or(DEFAULT_LIMIT),
                query.offset.unwrap_or(0),
                radius,
            )
            .await?;
        Ok(items.into_iter().map(to_response).collect())
    }

    /// "Iklan Saya" (Kelompok 3 Phase 2) — entry point ke "Kelola Pelamar".
    pub async fn list_my_jobs(
        &self,
        poster_id: Uuid,
        query: ListQuery,
    ) -> Result<Vec<IklanPekerjaanResponse>, anyhow::Error> {
        let items = self
            .repo
            .list_by_poster(
                poster_id,
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
                    Ok(_) => { /* valid */ }
                }
            }
        }

        let judul = ammonia::clean_text(&input.judul);
        let deskripsi = ammonia::clean_text(&input.deskripsi);
        let jam_kerja = input.jam_kerja.as_deref().map(ammonia::clean_text);
        let coords = self
            .geocode_lokasi(input.lokasi.as_deref(), input.region_id.as_deref())
            .await;
        let item = self
            .repo
            .create(CreatePekerjaanParams {
                poster_id,
                judul: &judul,
                perusahaan: &input.perusahaan,
                deskripsi: &deskripsi,
                tipe: &input.tipe,
                lokasi: input.lokasi.as_deref(),
                region_id: input.region_id.as_deref(),
                gaji_min: input.gaji_min,
                gaji_max: input.gaji_max,
                jam_kerja: jam_kerja.as_deref(),
                latitude: coords.map(|(lat, _)| lat),
                longitude: coords.map(|(_, lng)| lng),
            })
            .await?;
        Ok(to_response(item))
    }

    pub async fn delete(&self, id: Uuid, poster_id: Uuid) -> Result<bool, anyhow::Error> {
        self.repo.delete(id, poster_id).await
    }

    pub async fn update(
        &self,
        poster_id: Uuid,
        id: Uuid,
        input: UpdatePekerjaanInput,
    ) -> Result<IklanPekerjaanResponse, anyhow::Error> {
        // Rate limit: 30 req/15 menit per user untuk update iklan.
        if let Some(rl) = &self.rate_limiter {
            if !rl
                .allow("iklan_pekerjaan:update", &poster_id.to_string())
                .await
            {
                return Err(anyhow::anyhow!(
                    "terlalu banyak permintaan, coba lagi nanti"
                ));
            }
        }

        // Lifecycle guard: only allow update if moderation_status == Active.
        let existing = self
            .repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("iklan tidak ditemukan"))?;
        if existing.moderation_status != ModerationStatus::Active {
            return Err(anyhow::anyhow!(
                "iklan tidak dapat diedit karena status moderasi: {}",
                existing.moderation_status
            ));
        }

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
                    Ok(_) => { /* valid */ }
                }
            }
        }

        let judul = input.judul.map(|v| ammonia::clean_text(&v));
        let perusahaan = input.perusahaan.map(|v| ammonia::clean_text(&v));
        let deskripsi = input.deskripsi.map(|v| ammonia::clean_text(&v));
        let lokasi = input.lokasi.map(|v| ammonia::clean_text(&v));
        let jam_kerja = input.jam_kerja.map(|v| ammonia::clean_text(&v));

        // Re-geocode (F-1) hanya bila `lokasi`/`region_id` benar-benar diubah.
        let coords = if lokasi.is_some() || input.region_id.is_some() {
            self.geocode_lokasi(lokasi.as_deref(), input.region_id.as_deref())
                .await
        } else {
            None
        };

        let item = self
            .repo
            .update(UpdatePekerjaanParams {
                id,
                poster_id,
                judul: judul.as_deref(),
                perusahaan: perusahaan.as_deref(),
                deskripsi: deskripsi.as_deref(),
                lokasi: lokasi.as_deref(),
                region_id: input.region_id.as_deref(),
                gaji_min: input.gaji_min,
                gaji_max: input.gaji_max,
                tipe: input.tipe.as_deref(),
                jam_kerja: jam_kerja.as_deref(),
                foto_urls: input.foto_urls.as_deref(),
                is_active: input.is_active,
                latitude: coords.map(|(lat, _)| lat),
                longitude: coords.map(|(_, lng)| lng),
            })
            .await?;
        Ok(to_response(item))
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

    // ── Lamaran (F-3, Kelompok 3 Phase 1) ────────────────────────────────────

    /// P1.3: ajukan lamaran — 3 validasi eksplisit PRD §5.11.3.
    pub async fn lamar(
        &self,
        pelamar_id: Uuid,
        iklan_id: Uuid,
        input: LamarInput,
    ) -> Result<LamaranResponse, anyhow::Error> {
        let iklan = self
            .repo
            .find_by_id(iklan_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("iklan tidak ditemukan"))?;

        // Validasi #2: bukan iklan sendiri.
        if iklan.poster_id == pelamar_id {
            return Err(anyhow::anyhow!(
                "tidak dapat melamar pada iklan milik sendiri"
            ));
        }

        // Validasi #1: sudah punya Iklan Pekerja aktif.
        let has_pekerja = self
            .iklan_pekerja_client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("layanan verifikasi Iklan Pekerja tidak tersedia"))?
            .exists_active_for_poster(pelamar_id)
            .await
            .map_err(|e| anyhow::anyhow!("layanan verifikasi Iklan Pekerja gagal: {e}"))?;
        if !has_pekerja {
            return Err(anyhow::anyhow!(
                "Anda harus memiliki Iklan Pekerja aktif sebelum melamar pekerjaan"
            ));
        }

        // Validasi #3: tidak ada pekerjaan lain di rentang waktu yang sama.
        if self
            .repo
            .has_conflicting_lamaran(pelamar_id, input.tanggal, input.jam_mulai, input.jam_akhir)
            .await?
        {
            return Err(anyhow::anyhow!(
                "Anda sudah memiliki pekerjaan lain di rentang waktu yang sama"
            ));
        }

        let lamaran = self
            .repo
            .create_lamaran(CreateLamaranParams {
                iklan_id,
                pelamar_id,
                tanggal: input.tanggal,
                jam_mulai: input.jam_mulai,
                jam_akhir: input.jam_akhir,
                kuota_diambil: input.kuota_diambil.unwrap_or(1),
            })
            .await?;
        Ok(to_lamaran_response(lamaran))
    }

    /// P1.4: terima/tolak lamaran oleh pemilik iklan.
    pub async fn review_lamaran(
        &self,
        iklan_owner_id: Uuid,
        lamaran_id: Uuid,
        input: ReviewLamaranInput,
    ) -> Result<LamaranResponse, anyhow::Error> {
        self.repo
            .review_lamaran(lamaran_id, iklan_owner_id, input.approved)
            .await?
            .map(to_lamaran_response)
            .ok_or_else(|| anyhow::anyhow!("lamaran tidak ditemukan"))
    }

    /// P1.5: mulai bekerja — validasi geofence 50m dari lokasi iklan (F-1) sebelum
    /// transaksi atomik Lamaran→Proses + Iklan→SedangDikerjakan.
    pub async fn mulai_bekerja(
        &self,
        pelamar_id: Uuid,
        lamaran_id: Uuid,
        input: MulaiBekerjaInput,
    ) -> Result<LamaranResponse, anyhow::Error> {
        let lamaran = self
            .repo
            .find_lamaran_by_id(lamaran_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("lamaran tidak ditemukan"))?;
        if lamaran.pelamar_id != pelamar_id {
            return Err(anyhow::anyhow!("lamaran tidak ditemukan"));
        }

        let iklan = self
            .repo
            .find_by_id(lamaran.iklan_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("iklan tidak ditemukan"))?;
        let (lat, lng) = iklan.latitude.zip(iklan.longitude).ok_or_else(|| {
            anyhow::anyhow!("lokasi iklan tidak tersedia untuk validasi geofence")
        })?;
        if !common_geo::within_radius_km(
            lat,
            lng,
            input.latitude,
            input.longitude,
            GEOFENCE_RADIUS_KM,
        ) {
            return Err(anyhow::anyhow!(
                "Anda berada di luar radius 50m dari lokasi pekerjaan"
            ));
        }

        self.repo
            .mulai_bekerja(lamaran_id, pelamar_id)
            .await?
            .map(to_lamaran_response)
            .ok_or_else(|| anyhow::anyhow!("lamaran tidak dapat dimulai (status tidak sesuai)"))
    }

    /// P1.6: tandai pekerjaan selesai — transaksi atomik Lamaran→Selesai + Iklan→Selesai.
    /// F-19 (Kelompok 4 Phase 4): "proses pada iklan terkait selesai" — memicu
    /// auto-end Chat 2x24 jam untuk percakapan yang terhubung ke iklan ini.
    pub async fn tandai_selesai(
        &self,
        pelamar_id: Uuid,
        lamaran_id: Uuid,
    ) -> Result<LamaranResponse, anyhow::Error> {
        let lamaran = self
            .repo
            .tandai_selesai(lamaran_id, pelamar_id)
            .await?
            .ok_or_else(|| {
                anyhow::anyhow!("lamaran tidak dapat ditandai selesai (status tidak sesuai)")
            })?;

        // Fail-open (§4.5 backend I/O aman) — kegagalan menjadwalkan auto-end Chat
        // TIDAK boleh menggagalkan operasi utama (tandai selesai sudah tersimpan).
        if let Some(chat_client) = &self.chat_client {
            if let Err(e) = chat_client
                .schedule_auto_end_for_ad("pekerjaan", lamaran.iklan_id)
                .await
            {
                tracing::warn!(
                    error = ?e,
                    iklan_id = %lamaran.iklan_id,
                    "gagal menjadwalkan auto-end chat — dilewati (fail-open)"
                );
            }
        }

        Ok(to_lamaran_response(lamaran))
    }

    /// P1.7: pembatalan lamaran diterima oleh pemilik iklan — maks H-24 jam sebelum mulai.
    pub async fn batalkan_lamaran(
        &self,
        iklan_owner_id: Uuid,
        lamaran_id: Uuid,
        input: BatalkanLamaranInput,
    ) -> Result<LamaranResponse, anyhow::Error> {
        let lamaran = self
            .repo
            .find_lamaran_by_id(lamaran_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("lamaran tidak ditemukan"))?;

        let start = lamaran.tanggal.and_time(lamaran.jam_mulai);
        let hours_left = (start - Utc::now().naive_utc()).num_hours();
        if hours_left < CANCELLATION_CUTOFF_HOURS {
            return Err(anyhow::anyhow!(
                "pembatalan hanya dapat dilakukan maksimal {CANCELLATION_CUTOFF_HOURS} jam sebelum pekerjaan dimulai"
            ));
        }

        self.repo
            .batalkan_lamaran(lamaran_id, iklan_owner_id, &input.alasan)
            .await?
            .map(to_lamaran_response)
            .ok_or_else(|| anyhow::anyhow!("lamaran tidak ditemukan"))
    }

    /// P1.10: daftar lamaran untuk satu iklan — dipanggil pemilik iklan ("Kelola Pelamar").
    /// Dilengkapi ringkasan Iklan Pekerja aktif tiap pelamar (PRD §5.11.5, Phase 2) via
    /// satu batch lookup — Hazard #5, bukan N+1 per pelamar.
    pub async fn list_lamaran_for_iklan(
        &self,
        iklan_owner_id: Uuid,
        iklan_id: Uuid,
    ) -> Result<Vec<LamaranWithPelamarResponse>, anyhow::Error> {
        let iklan = self
            .repo
            .find_by_id(iklan_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("iklan tidak ditemukan"))?;
        if iklan.poster_id != iklan_owner_id {
            return Err(anyhow::anyhow!("iklan tidak ditemukan"));
        }
        let items = self.repo.list_lamaran_for_iklan(iklan_id).await?;

        let poster_ids: Vec<Uuid> = items.iter().map(|l| l.pelamar_id).collect();
        let summaries: std::collections::HashMap<
            Uuid,
            iklan_pekerja_service_client::IklanPekerjaSummary,
        > = match &self.iklan_pekerja_client {
            Some(client) if !poster_ids.is_empty() => client
                .get_active_summaries_for_posters(&poster_ids)
                .await
                .map(|v| v.into_iter().map(|s| (s.poster_id, s)).collect())
                .unwrap_or_default(),
            _ => Default::default(),
        };

        Ok(items
            .into_iter()
            .map(|l| {
                let summary = summaries.get(&l.pelamar_id);
                LamaranWithPelamarResponse {
                    id: l.id,
                    iklan_id: l.iklan_id,
                    pelamar_id: l.pelamar_id,
                    status: l.status,
                    tanggal: l.tanggal,
                    jam_mulai: l.jam_mulai,
                    jam_akhir: l.jam_akhir,
                    kuota_diambil: l.kuota_diambil,
                    alasan_batal: l.alasan_batal,
                    created_at: l.created_at,
                    updated_at: l.updated_at,
                    pelamar_nama: summary.map(|s| s.nama.clone()),
                    pelamar_iklan_pekerja_id: summary.map(|s| s.id),
                    pelamar_keahlian: summary.map(|s| s.keahlian.clone()),
                    pelamar_foto_url: summary.and_then(|s| s.foto_url.clone()),
                }
            })
            .collect())
    }

    /// P1.10: daftar lamaran milik satu pelamar — "Riwayat Aktifitas Pelamar".
    /// Dilengkapi judul/upah/tipe/alamat iklan per baris (PRD §5.11.4, Phase 2) via
    /// satu batch lookup (`find_by_ids`) — Hazard #5, bukan N+1.
    pub async fn list_lamaran_for_pelamar(
        &self,
        pelamar_id: Uuid,
    ) -> Result<Vec<LamaranWithIklanResponse>, anyhow::Error> {
        let items = self.repo.list_lamaran_for_pelamar(pelamar_id).await?;

        let iklan_ids: Vec<Uuid> = items.iter().map(|l| l.iklan_id).collect();
        let iklan_map: std::collections::HashMap<Uuid, crate::domain::entity::IklanPekerjaan> =
            if iklan_ids.is_empty() {
                Default::default()
            } else {
                self.repo
                    .find_by_ids(&iklan_ids)
                    .await?
                    .into_iter()
                    .map(|i| (i.id, i))
                    .collect()
            };

        Ok(items
            .into_iter()
            .map(|l| {
                let iklan = iklan_map.get(&l.iklan_id);
                LamaranWithIklanResponse {
                    id: l.id,
                    iklan_id: l.iklan_id,
                    pelamar_id: l.pelamar_id,
                    status: l.status,
                    tanggal: l.tanggal,
                    jam_mulai: l.jam_mulai,
                    jam_akhir: l.jam_akhir,
                    kuota_diambil: l.kuota_diambil,
                    alasan_batal: l.alasan_batal,
                    created_at: l.created_at,
                    updated_at: l.updated_at,
                    iklan_judul: iklan.map(|i| i.judul.clone()),
                    iklan_perusahaan: iklan.map(|i| i.perusahaan.clone()),
                    iklan_gaji_min: iklan.and_then(|i| i.gaji_min),
                    iklan_gaji_max: iklan.and_then(|i| i.gaji_max),
                    iklan_tipe: iklan.map(|i| i.tipe.clone()),
                    iklan_lokasi: iklan.and_then(|i| i.lokasi.clone()),
                    iklan_poster_id: iklan.map(|i| i.poster_id),
                }
            })
            .collect())
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
        region_id: e.region_id,
        gaji_min: e.gaji_min,
        gaji_max: e.gaji_max,
        tipe: e.tipe,
        jam_kerja: e.jam_kerja,
        foto_urls: e.foto_urls,
        is_active: e.is_active,
        moderation_status: e.moderation_status,
        status: e.status,
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
        region_id: e.region_id,
        gaji_min: e.gaji_min,
        gaji_max: e.gaji_max,
        tipe: e.tipe,
        jam_kerja: e.jam_kerja,
        foto_urls: e.foto_urls,
        is_active: e.is_active,
        moderation_status: e.moderation_status,
        status: e.status,
        deleted_at: e.deleted_at,
        created_at: e.created_at,
        updated_at: e.updated_at,
    }
}

fn to_lamaran_response(l: Lamaran) -> LamaranResponse {
    LamaranResponse {
        id: l.id,
        iklan_id: l.iklan_id,
        pelamar_id: l.pelamar_id,
        status: l.status,
        tanggal: l.tanggal,
        jam_mulai: l.jam_mulai,
        jam_akhir: l.jam_akhir,
        kuota_diambil: l.kuota_diambil,
        alasan_batal: l.alasan_batal,
        created_at: l.created_at,
        updated_at: l.updated_at,
    }
}
