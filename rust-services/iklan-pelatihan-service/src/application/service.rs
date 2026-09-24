use std::sync::Arc;
use uuid::Uuid;

use super::dto::{
    AdminIklanPelatihanResponse, AdminListQuery, BadgeEvidenceInput, BadgeListQuery, BadgeResponse,
    CommitBadgeSertifikatInput, CommitEnrollBuktiInput, CreateIklanPelatihanInput,
    EnrollEvidenceInput, EnrollmentListQuery, EnrollmentResponse, IklanPelatihanResponse,
    ListQuery, PelatihanListQuery, ReviewBadgeInput, ReviewEnrollmentInput, ReviewPelatihanInput,
    SuspendEvidenceInput, SuspendInput, SuspendResponse, SuspendResultItem,
    UpdateIklanPelatihanInput, UpdatePelatihanInput,
};
use crate::domain::cert_generator::{CertificateGenerator, CertificateInput};
use crate::domain::entity::{CreatedByRole, ModerationStatus, PelatihanStatus};
use crate::domain::repository::{
    CreatePelatihanParams, IklanPelatihanRepository, ListParams, PatchPelatihanParams,
    UpdatePelatihanParams, CSV_MAX, DEFAULT_LIMIT,
};
use common_geocoding::{GeocodeInput, GeocodingClient};
use common_rate_limit::RateLimiter;
use common_scheduler::{JobEnvelope, SchedulerClient};
use notification_service_client::NotificationClient;
use region_service_client::RegionClient;
use storage_service_client::StorageClient;
use user_service_client::UserClient;

/// Nama kategori storage — bukan hardcoded string literal.
pub mod storage_category {
    pub const SUSPENSION_EVIDENCE: &str = "iklan-suspension-evidence";
    pub const TRAINING_TRANSFER: &str = "training-transfer-evidence";
    pub const TRAINING_CERTIFICATE: &str = "training-certificate";
}

/// Radius default listing Iklan Pelatihan (PRD §5.13.1).
const DEFAULT_RADIUS_KM: f64 = 10.0;

pub struct IklanPelatihanService<R: IklanPelatihanRepository> {
    repo: Arc<R>,
    rate_limiter: Option<Arc<dyn RateLimiter>>,
    region_client: Option<Arc<dyn RegionClient>>,
    geocoding_client: Option<Arc<dyn GeocodingClient>>,
    scheduler_client: Option<Arc<SchedulerClient>>,
}

impl<R: IklanPelatihanRepository> IklanPelatihanService<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self {
            repo,
            rate_limiter: None,
            region_client: None,
            geocoding_client: None,
            scheduler_client: None,
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

    pub fn with_scheduler_client(mut self, sc: Arc<SchedulerClient>) -> Self {
        self.scheduler_client = Some(sc);
        self
    }

    /// Jadwalkan satu tugas otomatis Bab 10 (F-32) — fail-open: gagal menjadwalkan
    /// (`scheduler_client` tidak terpasang, atau Redis tidak tersedia) TIDAK PERNAH
    /// menggagalkan operasi utama (create/enroll), hanya `warn!` (§4.5 backend I/O aman).
    async fn schedule_job(
        &self,
        job_type: &str,
        payload: serde_json::Value,
        execute_after: chrono::DateTime<chrono::Utc>,
    ) {
        let Some(scheduler) = &self.scheduler_client else {
            return;
        };
        let envelope = JobEnvelope::new(job_type, payload, execute_after);
        if let Err(e) = scheduler.schedule(&envelope).await {
            tracing::warn!(
                job_type,
                error = ?e,
                "gagal menjadwalkan tugas otomatis Bab 10 — dilewati (fail-open)"
            );
        }
    }

    /// Jadwalkan 3 tugas otomatis Bab 10 sekaligus saat pelatihan dibuat (P6.4, P6.6,
    /// P6.7) — dipanggil dari `create_user`/`create_admin`. Setiap handler mengecek
    /// ULANG status/kondisi terkini saat dieksekusi (idempoten) — menjadwalkan di sini
    /// TIDAK berarti tindakan pasti dijalankan, hanya "cek nanti pada waktunya".
    async fn schedule_pelatihan_lifecycle_jobs(
        &self,
        pelatihan_id: Uuid,
        tanggal_mulai: Option<chrono::DateTime<chrono::Utc>>,
        tanggal_selesai: Option<chrono::DateTime<chrono::Utc>>,
    ) {
        if let Some(mulai) = tanggal_mulai {
            // P6.4: H-8 jam sebelum mulai, masih verifikasi → batalkan otomatis.
            self.schedule_job(
                crate::application::scheduled_jobs::JOB_CANCEL_UNVERIFIED,
                serde_json::json!({ "pelatihan_id": pelatihan_id }),
                mulai - chrono::Duration::hours(8),
            )
            .await;
        }
        if let Some(selesai) = tanggal_selesai {
            // P6.6: 1x24 jam setelah berakhir, belum ditekan Akhiri → tandai Selesai.
            self.schedule_job(
                crate::application::scheduled_jobs::JOB_MARK_SELESAI,
                serde_json::json!({ "pelatihan_id": pelatihan_id }),
                selesai + chrono::Duration::hours(24),
            )
            .await;
            // P6.7: 3x24 jam setelah selesai, badge belum diajukan → suspend penyelenggara.
            self.schedule_job(
                crate::application::scheduled_jobs::JOB_CHECK_BADGE_DEADLINE,
                serde_json::json!({ "pelatihan_id": pelatihan_id }),
                selesai + chrono::Duration::days(3),
            )
            .await;
        }
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
                tracing::warn!(error = ?e, "geocoding gagal — pelatihan disimpan tanpa koordinat");
                None
            }
        }
    }

    // ── Public listing ───────────────────────────────────────────────────

    pub async fn list(&self, q: ListQuery) -> Result<Vec<IklanPelatihanResponse>, anyhow::Error> {
        // Filter radius (F-1, F-14, PRD §5.13.1): aktif hanya bila KEDUA lat/lng dikirim.
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

    pub async fn get(&self, id: Uuid) -> Result<IklanPelatihanResponse, anyhow::Error> {
        self.repo
            .find_by_id(id)
            .await?
            .map(to_resp)
            .ok_or_else(|| anyhow::anyhow!("tidak ditemukan"))
    }

    // ── Create ──────────────────────────────────────────────────────────

    /// User create: `verifikasi_tertunda`, needs admin review.
    pub async fn create_user(
        &self,
        poster_id: Uuid,
        input: CreateIklanPelatihanInput,
    ) -> Result<IklanPelatihanResponse, anyhow::Error> {
        // Rate limit: 20 req/15 menit per user
        if let Some(rl) = &self.rate_limiter {
            if !rl
                .allow("iklan_pelatihan:create", &poster_id.to_string())
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
        let tanggal_mulai = input.tanggal_mulai;
        let tanggal_selesai = input.tanggal_selesai;

        let created = self
            .repo
            .create(CreatePelatihanParams {
                poster_id,
                judul: &sanitize(&input.judul),
                penyelenggara: &sanitize(&input.penyelenggara),
                deskripsi: &sanitize(&input.deskripsi),
                lokasi: input.lokasi.as_deref(),
                region_id: input.region_id.as_deref(),
                harga: input.harga,
                tanggal_mulai,
                tanggal_selesai,
                created_by_role: CreatedByRole::User.as_str(),
                initial_status: PelatihanStatus::VerifikasiTertunda.as_str(),
                jumlah_peserta: input.jumlah_peserta,
                latitude: coords.map(|(lat, _)| lat),
                longitude: coords.map(|(_, lng)| lng),
                bank_name: &sanitize(&input.bank_name),
                bank_account_number: &sanitize(&input.bank_account_number),
                bank_account_holder_name: &sanitize(&input.bank_account_holder_name),
                signature_object_key: input.signature_object_key.as_deref(),
            })
            .await?;
        self.schedule_pelatihan_lifecycle_jobs(created.id, tanggal_mulai, tanggal_selesai)
            .await;
        Ok(to_resp(created))
    }

    /// Admin create: auto-approve → `verifikasi_diterima`.
    pub async fn create_admin(
        &self,
        admin_id: Uuid,
        input: CreateIklanPelatihanInput,
    ) -> Result<IklanPelatihanResponse, anyhow::Error> {
        let coords = self
            .geocode_lokasi(input.lokasi.as_deref(), input.region_id.as_deref())
            .await;
        let tanggal_mulai = input.tanggal_mulai;
        let tanggal_selesai = input.tanggal_selesai;

        let created = self
            .repo
            .create(CreatePelatihanParams {
                poster_id: admin_id,
                judul: &sanitize(&input.judul),
                penyelenggara: &sanitize(&input.penyelenggara),
                deskripsi: &sanitize(&input.deskripsi),
                lokasi: input.lokasi.as_deref(),
                region_id: input.region_id.as_deref(),
                harga: input.harga,
                tanggal_mulai,
                tanggal_selesai,
                created_by_role: CreatedByRole::Admin.as_str(),
                initial_status: PelatihanStatus::VerifikasiDiterima.as_str(),
                jumlah_peserta: input.jumlah_peserta,
                latitude: coords.map(|(lat, _)| lat),
                longitude: coords.map(|(_, lng)| lng),
                bank_name: &sanitize(&input.bank_name),
                bank_account_number: &sanitize(&input.bank_account_number),
                bank_account_holder_name: &sanitize(&input.bank_account_holder_name),
                signature_object_key: input.signature_object_key.as_deref(),
            })
            .await?;
        self.schedule_pelatihan_lifecycle_jobs(created.id, tanggal_mulai, tanggal_selesai)
            .await;
        Ok(to_resp(created))
    }

    // ── Delete ──────────────────────────────────────────────────────────

    pub async fn delete(&self, id: Uuid, poster_id: Uuid) -> Result<bool, anyhow::Error> {
        self.repo.delete(id, poster_id).await
    }

    // ── PATCH (user-level partial update) ───────────────────────────────

    pub async fn update(
        &self,
        poster_id: Uuid,
        id: Uuid,
        input: UpdatePelatihanInput,
    ) -> Result<IklanPelatihanResponse, anyhow::Error> {
        // 1. Rate limit
        if let Some(rl) = &self.rate_limiter {
            if !rl
                .allow("iklan_pelatihan:update", &poster_id.to_string())
                .await
            {
                return Err(anyhow::anyhow!(
                    "terlalu banyak permintaan, coba lagi nanti"
                ));
            }
        }

        // 2. find_by_id + ownership check
        let existing = self
            .repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("tidak ditemukan"))?;

        if existing.poster_id != poster_id {
            return Err(anyhow::anyhow!("tidak ditemukan"));
        }

        // 3. Lifecycle guard: hanya bisa diubah jika moderation_status == Active
        //    DAN status IN (VerifikasiDiterima, PelatihanBelumDimulai)
        if existing.moderation_status != ModerationStatus::Active {
            return Err(anyhow::anyhow!("Iklan sedang ditangguhkan"));
        }
        match existing.status {
            PelatihanStatus::VerifikasiDiterima | PelatihanStatus::PelatihanBelumDimulai => {}
            _ => {
                return Err(anyhow::anyhow!(
                    "Pelatihan tidak dapat diubah pada status saat ini"
                ));
            }
        }

        // 4. Sanitasi
        let sanitized_judul = input.judul.as_ref().map(|v| sanitize(v));
        let sanitized_penyelenggara = input.penyelenggara.as_ref().map(|v| sanitize(v));
        let sanitized_deskripsi = input.deskripsi.as_ref().map(|v| sanitize(v));
        let sanitized_lokasi = input.lokasi.as_ref().map(|v| sanitize(v));
        let sanitized_region_id = input.region_id.as_ref().map(|v| sanitize(v));
        let sanitized_bank_name = input.bank_name.as_ref().map(|v| sanitize(v));
        let sanitized_bank_account_number = input.bank_account_number.as_ref().map(|v| sanitize(v));
        let sanitized_bank_account_holder_name =
            input.bank_account_holder_name.as_ref().map(|v| sanitize(v));

        // 5. Region validation
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

        // Re-geocode (F-1) hanya bila `lokasi`/`region_id` benar-benar diubah.
        let coords = if sanitized_lokasi.is_some() || sanitized_region_id.is_some() {
            self.geocode_lokasi(sanitized_lokasi.as_deref(), sanitized_region_id.as_deref())
                .await
        } else {
            None
        };

        // 6. Build optional params and call repo.update()
        let params = PatchPelatihanParams {
            judul: sanitized_judul,
            penyelenggara: sanitized_penyelenggara,
            deskripsi: sanitized_deskripsi,
            lokasi: sanitized_lokasi,
            region_id: sanitized_region_id,
            harga: input.harga,
            tanggal_mulai: input.tanggal_mulai,
            tanggal_selesai: input.tanggal_selesai,
            foto_urls: input.foto_urls,
            jumlah_peserta: input.jumlah_peserta,
            is_active: input.is_active,
            latitude: coords.map(|(lat, _)| lat),
            longitude: coords.map(|(_, lng)| lng),
            bank_name: sanitized_bank_name,
            bank_account_number: sanitized_bank_account_number,
            bank_account_holder_name: sanitized_bank_account_holder_name,
            signature_object_key: input.signature_object_key,
        };

        let updated = self
            .repo
            .update(id, poster_id, params)
            .await?
            .ok_or_else(|| anyhow::anyhow!("tidak ditemukan"))?;

        Ok(to_resp(updated))
    }

    // ── Admin: pelatihan listing ────────────────────────────────────────

    fn build_list_params(query: PelatihanListQuery) -> ListParams {
        ListParams {
            q: query.q,
            filter_column: String::from("status"),
            filter_value: query.status,
            sort_by: query.sort_by,
            sort_dir: query.sort_dir,
            limit: query.limit.unwrap_or(DEFAULT_LIMIT),
            offset: query.offset.unwrap_or(0),
        }
    }

    pub async fn admin_pelatihan_list(
        &self,
        query: PelatihanListQuery,
    ) -> Result<(Vec<AdminIklanPelatihanResponse>, i64), anyhow::Error> {
        let result = self
            .repo
            .admin_pelatihan_list(Self::build_list_params(query))
            .await?;
        Ok((
            result.items.into_iter().map(to_admin_resp).collect(),
            result.total,
        ))
    }

    pub async fn admin_pelatihan_export_csv(
        &self,
        query: PelatihanListQuery,
    ) -> Result<Vec<AdminIklanPelatihanResponse>, anyhow::Error> {
        let mut params = Self::build_list_params(query);
        params.limit = CSV_MAX;
        params.offset = 0;
        let items = self.repo.admin_pelatihan_list_all(params).await?;
        Ok(items.into_iter().map(to_admin_resp).collect())
    }

    // ── Admin: update pelatihan ─────────────────────────────────────────

    pub async fn admin_update_pelatihan(
        &self,
        admin_id: Uuid,
        id: Uuid,
        input: UpdateIklanPelatihanInput,
    ) -> Result<AdminIklanPelatihanResponse, anyhow::Error> {
        let existing = self
            .repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("tidak ditemukan"))?;
        if existing.poster_id != admin_id {
            return Err(anyhow::anyhow!("tidak ditemukan"));
        }
        if existing.created_by_role != CreatedByRole::Admin {
            return Err(anyhow::anyhow!(
                "Pelatihan milik pengguna tidak dapat disunting"
            ));
        }
        let coords = self
            .geocode_lokasi(input.lokasi.as_deref(), input.region_id.as_deref())
            .await;
        let updated = self
            .repo
            .update_pelatihan(UpdatePelatihanParams {
                id,
                poster_id: admin_id,
                judul: &sanitize(&input.judul),
                penyelenggara: &sanitize(&input.penyelenggara),
                deskripsi: &sanitize(&input.deskripsi),
                lokasi: input.lokasi.as_deref(),
                region_id: input.region_id.as_deref(),
                harga: input.harga,
                tanggal_mulai: input.tanggal_mulai,
                tanggal_selesai: input.tanggal_selesai,
                jumlah_peserta: input.jumlah_peserta,
                latitude: coords.map(|(lat, _)| lat),
                longitude: coords.map(|(_, lng)| lng),
                bank_name: &sanitize(&input.bank_name),
                bank_account_number: &sanitize(&input.bank_account_number),
                bank_account_holder_name: &sanitize(&input.bank_account_holder_name),
                signature_object_key: input.signature_object_key.as_deref(),
            })
            .await?
            .ok_or_else(|| anyhow::anyhow!("tidak ditemukan"))?;
        Ok(to_admin_resp(updated))
    }

    // ── Admin: cancel pelatihan ─────────────────────────────────────────

    pub async fn admin_cancel_pelatihan(
        &self,
        admin_id: Uuid,
        id: Uuid,
    ) -> Result<bool, anyhow::Error> {
        let existing = self
            .repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("tidak ditemukan"))?;
        if existing.poster_id != admin_id {
            return Ok(false);
        }
        if existing.created_by_role != CreatedByRole::Admin {
            return Err(anyhow::anyhow!(
                "Pelatihan milik pengguna tidak dapat dibatalkan oleh admin"
            ));
        }
        self.repo.soft_delete_pelatihan(id, admin_id).await
    }

    // ── Admin: review pelatihan ─────────────────────────────────────────

    pub async fn admin_review_pelatihan(
        &self,
        admin_id: Uuid,
        id: Uuid,
        input: ReviewPelatihanInput,
        notifier: Option<&dyn NotificationClient>,
        auth_client: Option<&dyn auth_service_client::AuthClient>,
    ) -> Result<AdminIklanPelatihanResponse, anyhow::Error> {
        validate_reject_note(input.approved, input.review_note.as_deref())?;

        let existing = self
            .repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("tidak ditemukan"))?;

        if !existing.status.can_review() {
            return Err(anyhow::anyhow!("Pelatihan sudah diverifikasi sebelumnya"));
        }
        if existing.created_by_role != CreatedByRole::User {
            return Err(anyhow::anyhow!(
                "Pelatihan ini dibuat oleh admin, tidak memerlukan review"
            ));
        }

        let updated = self
            .repo
            .review_pelatihan(id, input.approved, input.review_note.as_deref(), admin_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("tidak ditemukan atau sudah diverifikasi"))?;

        notify_pelatihan_review(
            notifier,
            auth_client,
            existing.poster_id,
            &existing.judul,
            existing.id,
            input.approved,
            input.review_note.as_deref(),
        )
        .await;

        Ok(to_admin_resp(updated))
    }

    // ── Admin: moderation list (existing) ───────────────────────────────

    pub async fn admin_list(
        &self,
        query: AdminListQuery,
    ) -> Result<(Vec<AdminIklanPelatihanResponse>, i64), anyhow::Error> {
        let params = crate::domain::repository::AdminListParams {
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
    ) -> Result<Vec<AdminIklanPelatihanResponse>, anyhow::Error> {
        let params = crate::domain::repository::AdminListParams {
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

    // ── Evidence & Suspend ──────────────────────────────────────────────

    pub async fn request_suspend_evidence(
        &self,
        storage: &dyn StorageClient,
        admin_id: Uuid,
        input: SuspendEvidenceInput,
    ) -> Result<storage_service_client::UploadPermission, anyhow::Error> {
        request_storage_upload(
            storage,
            storage_category::SUSPENSION_EVIDENCE,
            admin_id,
            &input.mime,
            input.size_bytes,
        )
        .await
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
            results.push(SuspendResultItem {
                iklan_id: *iklan_id,
                success: suspended,
                error: if suspended {
                    None
                } else {
                    Some("iklan tidak ditemukan atau sudah tersuspensi".into())
                },
            });
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
                        iklan.judul, s.iklan_id, s.reason
                    );
                    send_notification(
                        notifier,
                        &NotifyPayload {
                            recipient_id: owner_id,
                            title,
                            body: &body,
                            data: &json!({ "type": "iklan_suspended", "iklan_id": s.iklan_id.to_string(), "is_permanent": s.is_permanent }),
                            email: Some(NotifyEmail { to: &email, subject: title, body: &body }),
                        },
                    )
                    .await;
                }
            }
        }

        Ok(SuspendResponse { results })
    }

    pub async fn expire_temporary_suspensions(&self) -> Result<u64, anyhow::Error> {
        self.repo.expire_temporary_suspensions().await
    }

    // ── Enrollment ──────────────────────────────────────────────────────

    pub async fn request_enroll_evidence(
        &self,
        storage: &dyn StorageClient,
        user_id: Uuid,
        input: EnrollEvidenceInput,
    ) -> Result<storage_service_client::UploadPermission, anyhow::Error> {
        request_storage_upload(
            storage,
            storage_category::TRAINING_TRANSFER,
            user_id,
            &input.mime,
            input.size_bytes,
        )
        .await
    }

    pub async fn create_enrollment(
        &self,
        pelatihan_id: Uuid,
        user_id: Uuid,
    ) -> Result<EnrollmentResponse, anyhow::Error> {
        let pelatihan = self
            .repo
            .find_by_id(pelatihan_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Pelatihan tidak ditemukan"))?;
        if pelatihan.deleted_at.is_some() {
            return Err(anyhow::anyhow!("Pelatihan tidak ditemukan"));
        }
        if !matches!(
            pelatihan.status,
            PelatihanStatus::VerifikasiDiterima
                | PelatihanStatus::PelatihanBelumDimulai
                | PelatihanStatus::PelatihanBerjalan
        ) {
            return Err(anyhow::anyhow!(
                "Pelatihan belum tersedia untuk pendaftaran"
            ));
        }
        // P6.5 (Bab 10, F-32): tutup pendaftaran 30 menit sebelum mulai — lazy check
        // (bukan job aktif, konsisten dengan pola cooldown KYC/OTP existing) karena
        // tidak ada state tersimpan untuk "ditutup"/"dibuka" yang perlu diubah oleh
        // proses background; cukup dibandingkan langsung terhadap `tanggal_mulai`.
        if let Some(mulai) = pelatihan.tanggal_mulai {
            if chrono::Utc::now() >= mulai - chrono::Duration::minutes(30) {
                return Err(anyhow::anyhow!(
                    "pendaftaran sudah ditutup (kurang dari 30 menit sebelum pelatihan dimulai)"
                ));
            }
        }
        let enrollment = self.repo.create_enrollment(pelatihan_id, user_id).await?;
        // P6.8: 1x24 jam setelah kode pembayaran terbit (= saat enrollment dibuat),
        // bukti belum dikirim → batalkan pendaftaran.
        self.schedule_job(
            crate::application::scheduled_jobs::JOB_CANCEL_ENROLLMENT_UNPAID,
            serde_json::json!({ "enrollment_id": enrollment.id }),
            enrollment.created_at + chrono::Duration::hours(24),
        )
        .await;
        Ok(to_enrollment_resp(enrollment))
    }

    pub async fn commit_enrollment_bukti(
        &self,
        id: Uuid,
        user_id: Uuid,
        input: CommitEnrollBuktiInput,
    ) -> Result<EnrollmentResponse, anyhow::Error> {
        let updated = self
            .repo
            .commit_enrollment_bukti(id, user_id, &input.object_key)
            .await?
            .ok_or_else(|| {
                anyhow::anyhow!("Enrollment tidak ditemukan atau bukti sudah diunggah")
            })?;
        Ok(to_enrollment_resp(updated))
    }

    pub async fn get_enrollment(&self, id: Uuid) -> Result<EnrollmentResponse, anyhow::Error> {
        self.repo
            .find_enrollment_by_id(id)
            .await?
            .map(to_enrollment_resp)
            .ok_or_else(|| anyhow::anyhow!("tidak ditemukan"))
    }

    // ── Admin: enrollment listing ───────────────────────────────────────

    fn build_enrollment_list_params(query: EnrollmentListQuery) -> ListParams {
        ListParams {
            q: query.q,
            filter_column: String::from("e.status"),
            filter_value: query.status,
            sort_by: query.sort_by,
            sort_dir: query.sort_dir,
            limit: query.limit.unwrap_or(DEFAULT_LIMIT),
            offset: query.offset.unwrap_or(0),
        }
    }

    pub async fn admin_enrollment_list(
        &self,
        query: EnrollmentListQuery,
    ) -> Result<(Vec<EnrollmentResponse>, i64), anyhow::Error> {
        let result = self
            .repo
            .admin_enrollment_list(Self::build_enrollment_list_params(query))
            .await?;
        Ok((
            result.items.into_iter().map(to_enrollment_resp).collect(),
            result.total,
        ))
    }

    pub async fn admin_enrollment_export_csv(
        &self,
        query: EnrollmentListQuery,
    ) -> Result<Vec<EnrollmentResponse>, anyhow::Error> {
        let mut params = Self::build_enrollment_list_params(query);
        params.limit = CSV_MAX;
        params.offset = 0;
        let items = self.repo.admin_enrollment_list_all(params).await?;
        Ok(items.into_iter().map(to_enrollment_resp).collect())
    }

    // ── Admin: review enrollment ─────────────────────────────────────────

    pub async fn admin_review_enrollment(
        &self,
        admin_id: Uuid,
        id: Uuid,
        input: ReviewEnrollmentInput,
        notifier: Option<&dyn NotificationClient>,
        auth_client: Option<&dyn auth_service_client::AuthClient>,
    ) -> Result<EnrollmentResponse, anyhow::Error> {
        validate_reject_note(input.approved, input.review_note.as_deref())?;

        let existing = self
            .repo
            .find_enrollment_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("tidak ditemukan"))?;

        if !existing.status.can_review() {
            return Err(anyhow::anyhow!("Pendaftaran sudah diverifikasi sebelumnya"));
        }

        let updated = self
            .repo
            .review_enrollment(id, input.approved, input.review_note.as_deref(), admin_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("tidak ditemukan atau sudah diverifikasi"))?;

        notify_review(
            notifier,
            auth_client,
            existing.user_id,
            "Pendaftaran pelatihan Anda telah",
            format!(
                "Pendaftaran Anda pada pelatihan (ID: {}) telah {}. {}",
                existing.pelatihan_id,
                if input.approved {
                    "disetujui"
                } else {
                    "ditolak"
                },
                input
                    .review_note
                    .as_deref()
                    .map(|n| format!("Catatan: {}", n))
                    .unwrap_or_default()
            ),
            input.approved,
            &json!({ "type": "enrollment_reviewed", "enrollment_id": id.to_string() }),
        )
        .await;

        Ok(to_enrollment_resp(updated))
    }

    // ── Badge ───────────────────────────────────────────────────────────

    pub async fn request_badge_evidence(
        &self,
        storage: &dyn StorageClient,
        user_id: Uuid,
        input: BadgeEvidenceInput,
    ) -> Result<storage_service_client::UploadPermission, anyhow::Error> {
        request_storage_upload(
            storage,
            storage_category::TRAINING_CERTIFICATE,
            user_id,
            &input.mime,
            input.size_bytes,
        )
        .await
    }

    pub async fn create_badge(
        &self,
        pelatihan_id: Uuid,
        user_id: Uuid,
    ) -> Result<BadgeResponse, anyhow::Error> {
        let pelatihan = self
            .repo
            .find_by_id(pelatihan_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Pelatihan tidak ditemukan"))?;
        if pelatihan.deleted_at.is_some() {
            return Err(anyhow::anyhow!("Pelatihan tidak ditemukan"));
        }
        let badge = self.repo.create_badge(pelatihan_id, user_id).await?;
        Ok(to_badge_resp(badge))
    }

    pub async fn commit_badge_sertifikat(
        &self,
        id: Uuid,
        user_id: Uuid,
        input: CommitBadgeSertifikatInput,
    ) -> Result<BadgeResponse, anyhow::Error> {
        let updated = self
            .repo
            .commit_badge_sertifikat(id, user_id, &input.object_key)
            .await?
            .ok_or_else(|| {
                anyhow::anyhow!("Badge tidak ditemukan atau sertifikat sudah diunggah")
            })?;
        Ok(to_badge_resp(updated))
    }

    pub async fn get_badge(&self, id: Uuid) -> Result<BadgeResponse, anyhow::Error> {
        self.repo
            .find_badge_by_id(id)
            .await?
            .map(to_badge_resp)
            .ok_or_else(|| anyhow::anyhow!("tidak ditemukan"))
    }

    // ── Admin: badge listing ────────────────────────────────────────────

    fn build_badge_list_params(query: BadgeListQuery) -> ListParams {
        ListParams {
            q: query.q,
            filter_column: String::from("b.status"),
            filter_value: query.status,
            sort_by: query.sort_by,
            sort_dir: query.sort_dir,
            limit: query.limit.unwrap_or(DEFAULT_LIMIT),
            offset: query.offset.unwrap_or(0),
        }
    }

    pub async fn admin_badge_list(
        &self,
        query: BadgeListQuery,
    ) -> Result<(Vec<BadgeResponse>, i64), anyhow::Error> {
        let result = self
            .repo
            .admin_badge_list(Self::build_badge_list_params(query))
            .await?;
        Ok((
            result.items.into_iter().map(to_badge_resp).collect(),
            result.total,
        ))
    }

    pub async fn admin_badge_export_csv(
        &self,
        query: BadgeListQuery,
    ) -> Result<Vec<BadgeResponse>, anyhow::Error> {
        let mut params = Self::build_badge_list_params(query);
        params.limit = CSV_MAX;
        params.offset = 0;
        let items = self.repo.admin_badge_list_all(params).await?;
        Ok(items.into_iter().map(to_badge_resp).collect())
    }

    // ── Admin: review badge ─────────────────────────────────────────────

    #[allow(clippy::too_many_arguments)]
    pub async fn admin_review_badge(
        &self,
        admin_id: Uuid,
        id: Uuid,
        input: ReviewBadgeInput,
        notifier: Option<&dyn NotificationClient>,
        auth_client: Option<&dyn auth_service_client::AuthClient>,
        storage: Option<&dyn StorageClient>,
        user_client: Option<&dyn UserClient>,
        cert_generator: Option<&dyn CertificateGenerator>,
    ) -> Result<BadgeResponse, anyhow::Error> {
        validate_reject_note(input.approved, input.review_note.as_deref())?;

        let existing = self
            .repo
            .find_badge_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("tidak ditemukan"))?;

        if !existing.status.can_review() {
            return Err(anyhow::anyhow!("Badge sudah diverifikasi sebelumnya"));
        }

        let mut updated = self
            .repo
            .review_badge(id, input.approved, input.review_note.as_deref(), admin_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("tidak ditemukan atau sudah diverifikasi"))?;

        // F-12 (Kelompok 6 P6): generate sertifikat PDF resmi otomatis saat
        // approve. Fail-open — kegagalan apa pun (storage/font/pelatihan
        // tidak ketemu) TIDAK boleh menggagalkan approval, cukup di-log;
        // sertifikat lama (hasil commit peserta, bila ada) dibiarkan.
        let mut sertifikat_read_url = None;
        if input.approved {
            if let (Some(storage), Some(cert_gen)) = (storage, cert_generator) {
                match self
                    .generate_and_store_certificate(&existing, storage, user_client, cert_gen)
                    .await
                {
                    Ok((new_key, read_url)) => {
                        match self.repo.set_badge_sertifikat(id, &new_key).await {
                            Ok(Some(b)) => {
                                updated = b;
                                sertifikat_read_url = read_url;
                            }
                            Ok(None) => {
                                tracing::warn!(badge_id = %id, "gagal simpan sertifikat: badge tidak ditemukan")
                            }
                            Err(e) => {
                                tracing::warn!(badge_id = %id, error = ?e, "gagal simpan sertifikat")
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!(badge_id = %id, error = ?e, "gagal generate sertifikat otomatis")
                    }
                }
            }
        }

        notify_review(
            notifier,
            auth_client,
            existing.user_id,
            "Sertifikat pelatihan Anda telah",
            format!(
                "Pengajuan sertifikat Anda untuk pelatihan (ID: {}) telah {}. {}{}",
                existing.pelatihan_id,
                if input.approved {
                    "disetujui"
                } else {
                    "ditolak"
                },
                input
                    .review_note
                    .as_deref()
                    .map(|n| format!("Catatan: {}", n))
                    .unwrap_or_default(),
                sertifikat_read_url
                    .map(|u| format!(" Unduh sertifikat: {}", u))
                    .unwrap_or_default(),
            ),
            input.approved,
            &json!({ "type": "badge_reviewed", "badge_id": id.to_string() }),
        )
        .await;

        Ok(to_badge_resp(updated))
    }

    /// Helper P6.3 — fetch pelatihan + nama peserta, generate PDF, upload ke
    /// storage. Dipisah dari `admin_review_badge` supaya method utama tidak
    /// jadi God Function (CLAUDE.md §4.7).
    async fn generate_and_store_certificate(
        &self,
        badge: &crate::domain::entity::PelatihanBadge,
        storage: &dyn StorageClient,
        user_client: Option<&dyn UserClient>,
        cert_gen: &dyn CertificateGenerator,
    ) -> Result<(String, Option<String>), anyhow::Error> {
        let pelatihan = self
            .repo
            .find_by_id(badge.pelatihan_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("pelatihan terkait tidak ditemukan"))?;

        let peserta_nama = match user_client {
            Some(uc) => uc
                .get_user_summary(badge.user_id)
                .await
                .map(|s| s.username)
                .unwrap_or_else(|_| badge.user_id.to_string()),
            None => badge.user_id.to_string(),
        };

        let signature_bytes = match &pelatihan.signature_object_key {
            Some(key) => storage.download_bytes(key).await.ok(),
            None => None,
        };

        let pdf_bytes = cert_gen.generate(CertificateInput {
            peserta_nama: &peserta_nama,
            judul_pelatihan: &pelatihan.judul,
            nama_perusahaan: &pelatihan.penyelenggara,
            alamat_perusahaan: pelatihan.lokasi.as_deref().unwrap_or("-"),
            // Tidak ada field `pejabat_nama` terpisah di entity — pakai
            // `bank_account_holder_name` (pemegang rekening perusahaan,
            // proxy wajar untuk pejabat berwenang) sebagai fallback ke nama
            // perusahaan bila kosong (data historis pra-P1).
            pejabat_nama: pelatihan
                .bank_account_holder_name
                .as_deref()
                .unwrap_or(&pelatihan.penyelenggara),
            signature_image: signature_bytes.as_deref(),
        })?;

        let object_key = storage
            .upload_bytes(
                storage_category::TRAINING_CERTIFICATE,
                badge.user_id,
                pdf_bytes,
                "application/pdf",
            )
            .await?;

        let read_url = storage.request_download(&object_key).await.ok();

        Ok((object_key, read_url))
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Shared helper functions — mencegah duplikasi kode notifikasi & validasi
// ═══════════════════════════════════════════════════════════════════════════

use serde_json::json;

#[inline]
fn sanitize(s: &str) -> String {
    ammonia::clean_text(s)
}

/// Validasi bahwa reject wajib punya alasan.
fn validate_reject_note(approved: bool, review_note: Option<&str>) -> Result<(), anyhow::Error> {
    if !approved && review_note.is_none_or(|s| s.trim().is_empty()) {
        return Err(anyhow::anyhow!("Alasan penolakan wajib diisi"));
    }
    Ok(())
}

/// Request presigned upload URL dari StorageClient.
async fn request_storage_upload(
    storage: &dyn StorageClient,
    category: &str,
    user_id: Uuid,
    mime: &str,
    size_bytes: u64,
) -> Result<storage_service_client::UploadPermission, anyhow::Error> {
    storage
        .request_upload(
            category,
            user_id,
            storage_service_client::FileInfo {
                mime: mime.to_string(),
                size_bytes,
            },
        )
        .await
        .map_err(|e| anyhow::anyhow!("storage error: {e}"))
}

/// Payload untuk `send_notification` — grouping untuk menghindari too_many_arguments.
struct NotifyPayload<'a> {
    recipient_id: Uuid,
    title: &'a str,
    body: &'a str,
    data: &'a serde_json::Value,
    email: Option<NotifyEmail<'a>>,
}

struct NotifyEmail<'a> {
    to: &'a str,
    subject: &'a str,
    body: &'a str,
}

/// Kirim push notification + email.
async fn send_notification(notifier: &dyn NotificationClient, payload: &NotifyPayload<'_>) {
    let _ = notifier
        .send(
            payload.recipient_id,
            notification_service_client::NotificationPayload {
                title: payload.title.to_string(),
                body: payload.body.to_string(),
                data: Some(payload.data.clone()),
            },
        )
        .await;
    if let Some(ref email) = payload.email {
        if !email.to.is_empty() {
            let _ = notifier
                .send_email(notification_service_client::EmailMessage {
                    to: email.to.to_string(),
                    subject: email.subject.to_string(),
                    body: email.body.to_string(),
                })
                .await;
        }
    }
}

/// Khusus pelatihan review — format khusus dengan `existing.judul`.
async fn notify_pelatihan_review(
    notifier: Option<&dyn NotificationClient>,
    auth_client: Option<&dyn auth_service_client::AuthClient>,
    poster_id: Uuid,
    judul: &str,
    pelatihan_id: Uuid,
    approved: bool,
    review_note: Option<&str>,
) {
    if let (Some(notifier), Some(auth_client)) = (notifier, auth_client) {
        let email = auth_client
            .get_account_email(poster_id)
            .await
            .unwrap_or_default();
        let (title, status_text) = if approved {
            ("Pelatihan Anda telah disetujui", "disetujui")
        } else {
            ("Pelatihan Anda telah ditolak", "ditolak")
        };
        let body = format!(
            "Pelatihan \"{}\" (ID: {}) telah {}. {}",
            judul,
            pelatihan_id,
            status_text,
            review_note
                .map(|n| format!("Catatan: {}", n))
                .unwrap_or_default()
        );
        send_notification(
            notifier,
            &NotifyPayload {
                recipient_id: poster_id,
                title,
                body: &body,
                data: &json!({ "type": "pelatihan_reviewed", "pelatihan_id": pelatihan_id.to_string(), "approved": approved }),
                email: Some(NotifyEmail { to: &email, subject: title, body: &body }),
            },
        )
        .await;
    }
}

/// Khusus enrollment + badge review — format umum tanpa `existing.judul`.
async fn notify_review(
    notifier: Option<&dyn NotificationClient>,
    auth_client: Option<&dyn auth_service_client::AuthClient>,
    recipient_id: Uuid,
    title_prefix: &str,
    body: String,
    approved: bool,
    data: &serde_json::Value,
) {
    if let (Some(notifier), Some(auth_client)) = (notifier, auth_client) {
        let email = auth_client
            .get_account_email(recipient_id)
            .await
            .unwrap_or_default();
        let (title, _status_text) = if approved {
            (format!("{} disetujui", title_prefix), "disetujui")
        } else {
            (format!("{} ditolak", title_prefix), "ditolak")
        };
        send_notification(
            notifier,
            &NotifyPayload {
                recipient_id,
                title: &title,
                body: &body,
                data,
                email: Some(NotifyEmail {
                    to: &email,
                    subject: &title,
                    body: &body,
                }),
            },
        )
        .await;
    }
}

// ── Mapper functions ─────────────────────────────────────────────────────

fn to_resp(e: crate::domain::entity::IklanPelatihan) -> IklanPelatihanResponse {
    IklanPelatihanResponse {
        id: e.id,
        poster_id: e.poster_id,
        judul: e.judul,
        penyelenggara: e.penyelenggara,
        deskripsi: e.deskripsi,
        lokasi: e.lokasi,
        region_id: e.region_id,
        harga: e.harga,
        tanggal_mulai: e.tanggal_mulai,
        tanggal_selesai: e.tanggal_selesai,
        foto_urls: e.foto_urls,
        is_active: e.is_active,
        moderation_status: e.moderation_status,
        status: e.status,
        created_by_role: e.created_by_role,
        jumlah_peserta: e.jumlah_peserta,
        created_at: e.created_at,
        bank_name: e.bank_name,
        bank_account_number: e.bank_account_number,
        bank_account_holder_name: e.bank_account_holder_name,
        signature_object_key: e.signature_object_key,
    }
}

fn to_admin_resp(e: crate::domain::entity::IklanPelatihan) -> AdminIklanPelatihanResponse {
    AdminIklanPelatihanResponse {
        id: e.id,
        poster_id: e.poster_id,
        judul: e.judul,
        penyelenggara: e.penyelenggara,
        deskripsi: e.deskripsi,
        lokasi: e.lokasi,
        region_id: e.region_id,
        harga: e.harga,
        tanggal_mulai: e.tanggal_mulai,
        tanggal_selesai: e.tanggal_selesai,
        foto_urls: e.foto_urls,
        is_active: e.is_active,
        moderation_status: e.moderation_status,
        status: e.status,
        created_by_role: e.created_by_role,
        jumlah_peserta: e.jumlah_peserta,
        deleted_at: e.deleted_at,
        created_at: e.created_at,
        updated_at: e.updated_at,
        bank_name: e.bank_name,
        bank_account_number: e.bank_account_number,
        bank_account_holder_name: e.bank_account_holder_name,
        signature_object_key: e.signature_object_key,
    }
}

fn to_enrollment_resp(e: crate::domain::entity::PelatihanEnrollment) -> EnrollmentResponse {
    EnrollmentResponse {
        id: e.id,
        pelatihan_id: e.pelatihan_id,
        user_id: e.user_id,
        bukti_transfer_object_key: e.bukti_transfer_object_key,
        status: e.status,
        reviewed_by: e.reviewed_by,
        review_note: e.review_note,
        created_at: e.created_at,
        updated_at: e.updated_at,
        bukti_transfer_read_url: None,
    }
}

fn to_badge_resp(e: crate::domain::entity::PelatihanBadge) -> BadgeResponse {
    BadgeResponse {
        id: e.id,
        pelatihan_id: e.pelatihan_id,
        user_id: e.user_id,
        sertifikat_object_key: e.sertifikat_object_key,
        approved_at: e.approved_at,
        status: e.status,
        reviewed_by: e.reviewed_by,
        review_note: e.review_note,
        created_at: e.created_at,
        updated_at: e.updated_at,
        sertifikat_read_url: None,
    }
}
