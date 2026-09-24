use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::business_days::add_business_days;
use crate::domain::entity::Report;
use crate::domain::repository::{
    CreateReportParams, ReportListParams, ReportRepository, DEFAULT_LIMIT,
};
use auth_service_client::AuthClient;
use common_rate_limit::RateLimiter;
use iklan_barang_bekas_service_client::IklanBarangBekasClient;
use iklan_pekerja_service_client::IklanPekerjaClient;
use iklan_pekerjaan_service_client::IklanPekerjaanClient;
use region_service_client::RegionClient;
use report_service_client::{ReportAdType, ReportStatus, ReportTargetType, ReportType};
use user_service_client::UserClient;

use super::dto::{ReportDetailResponse, ReportListQuery, ReportResponse, ReporterDemographics};

/// Nama kategori storage — bukan hardcoded string literal.
pub mod storage_category {
    pub const REPORT_EVIDENCE: &str = "report-evidence";
}

/// SLA penanganan aduan — PRD §6.10 "maksimal 7 hari kerja", berlaku sama untuk
/// kedua jalur (Laporkan Iklan & Pelaporan Masalah), keputusan final klien B-8.
const SLA_BUSINESS_DAYS: i64 = 7;

/// Durasi fallback suspend sementara via approve-and-suspend (P9.1) untuk
/// `target_type=User` bila admin tidak memilih tanggal spesifik — UI existing
/// (pola `SuspendIklanPayload`/`SuspendPenggunaPayload`) memang tidak pernah
/// mengirim tanggal, hanya toggle sementara/permanen.
const DEFAULT_TEMP_SUSPEND_DAYS: i64 = 30;

pub struct ReportService<R: ReportRepository> {
    repo: Arc<R>,
    rate_limiter: Option<Arc<dyn RateLimiter>>,
    user_client: Option<Arc<dyn UserClient>>,
    region_client: Option<Arc<dyn RegionClient>>,
}

impl<R: ReportRepository> ReportService<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self {
            repo,
            rate_limiter: None,
            user_client: None,
            region_client: None,
        }
    }
    pub fn with_rate_limiter(mut self, rl: Arc<dyn RateLimiter>) -> Self {
        self.rate_limiter = Some(rl);
        self
    }
    pub fn with_user_client(mut self, uc: Arc<dyn UserClient>) -> Self {
        self.user_client = Some(uc);
        self
    }
    pub fn with_region_client(mut self, rc: Arc<dyn RegionClient>) -> Self {
        self.region_client = Some(rc);
        self
    }

    // ── User: create report — 2 jalur (P1.4) ───────────────────────────────

    /// Jalur "Laporkan Iklan" (PRD §5.10) — dari halaman detail Iklan
    /// Pekerjaan/Pekerja. Target WAJIB.
    pub async fn create_laporkan_iklan(
        &self,
        reporter_id: Uuid,
        target_type: ReportTargetType,
        target_ad_type: Option<ReportAdType>,
        target_id: Uuid,
        keterangan: String,
    ) -> Result<Report, anyhow::Error> {
        self.create_internal(
            reporter_id,
            ReportType::LaporkanIklan,
            Some(target_type),
            target_ad_type,
            Some(target_id),
            keterangan,
            None,
        )
        .await
    }

    /// Jalur "Pelaporan Masalah" (PRD §5.10) — dari proses yang gagal. Target
    /// opsional; bukti foto WAJIB divalidasi di handler (upload storage) sebelum
    /// method ini dipanggil — `evidence_object_key` di sini sudah hasil upload.
    pub async fn create_pelaporan_masalah(
        &self,
        reporter_id: Uuid,
        target_id: Option<Uuid>,
        keterangan: String,
        evidence_object_key: String,
    ) -> Result<Report, anyhow::Error> {
        self.create_internal(
            reporter_id,
            ReportType::PelaporanMasalah,
            target_id.map(|_| ReportTargetType::Iklan),
            None,
            target_id,
            keterangan,
            Some(evidence_object_key),
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn create_internal(
        &self,
        reporter_id: Uuid,
        report_type: ReportType,
        target_type: Option<ReportTargetType>,
        target_ad_type: Option<ReportAdType>,
        target_id: Option<Uuid>,
        keterangan: String,
        evidence_object_key: Option<String>,
    ) -> Result<Report, anyhow::Error> {
        // Rate limit: 10 req/15 menit per user
        if let Some(rl) = &self.rate_limiter {
            if !rl.allow("report:create", &reporter_id.to_string()).await {
                return Err(anyhow::anyhow!(
                    "terlalu banyak permintaan, coba lagi nanti"
                ));
            }
        }
        let due_date = add_business_days(chrono::Utc::now(), SLA_BUSINESS_DAYS);
        self.repo
            .create(CreateReportParams {
                reporter_id,
                report_type,
                target_type,
                target_id,
                target_ad_type,
                keterangan: &keterangan,
                evidence_object_key: evidence_object_key.as_deref(),
                due_date,
            })
            .await
    }

    // ── Admin: list reports ───────────────────────────────────────────────

    pub async fn admin_list(
        &self,
        query: ReportListQuery,
    ) -> Result<(Vec<ReportResponse>, i64), anyhow::Error> {
        let status = query.status.as_deref().and_then(ReportStatus::parse);
        let report_type = query.report_type.as_deref().and_then(ReportType::parse);
        let result = self
            .repo
            .list(ReportListParams {
                q: query.q,
                status,
                report_type,
                sort_dir: query.sort_dir,
                limit: query.limit.unwrap_or(DEFAULT_LIMIT),
                offset: query.offset.unwrap_or(0),
            })
            .await?;

        // P1.3: enrichment demografis batched (Hazard #5) — 1 lookup untuk seluruh halaman.
        let reporter_ids: Vec<Uuid> = result.items.iter().map(|r| r.reporter_id).collect();
        let demographics = self.enrich_demographics(&reporter_ids).await;

        let items = result
            .items
            .into_iter()
            .map(|r| {
                let demo = demographics.get(&r.reporter_id).cloned();
                to_report_resp(r, demo)
            })
            .collect();
        Ok((items, result.total))
    }

    // ── Admin: list all (for CSV) ─────────────────────────────────────────

    pub async fn admin_list_all(
        &self,
        query: ReportListQuery,
    ) -> Result<Vec<ReportResponse>, anyhow::Error> {
        let status = query.status.as_deref().and_then(ReportStatus::parse);
        let report_type = query.report_type.as_deref().and_then(ReportType::parse);
        // Gunakan limit besar untuk CSV; tetap aman karena volume aduan kecil-moderat.
        let result = self
            .repo
            .list(ReportListParams {
                q: query.q,
                status,
                report_type,
                sort_dir: query.sort_dir,
                limit: 10_000,
                offset: 0,
            })
            .await?;
        let reporter_ids: Vec<Uuid> = result.items.iter().map(|r| r.reporter_id).collect();
        let demographics = self.enrich_demographics(&reporter_ids).await;
        Ok(result
            .items
            .into_iter()
            .map(|r| {
                let demo = demographics.get(&r.reporter_id).cloned();
                to_report_resp(r, demo)
            })
            .collect())
    }

    // ── Admin: get detail ─────────────────────────────────────────────────

    pub async fn admin_get_detail(&self, id: Uuid) -> Result<ReportDetailResponse, anyhow::Error> {
        let report = self
            .repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("aduan tidak ditemukan"))?;
        let demographics = self.enrich_demographics(&[report.reporter_id]).await;
        let demo = demographics
            .get(&report.reporter_id)
            .cloned()
            .unwrap_or_default();
        Ok(to_detail_resp(report, demo))
    }

    // ── Admin: review ─────────────────────────────────────────────────────

    pub async fn admin_review(
        &self,
        id: Uuid,
        approved: bool,
        action_note: String,
        reviewed_by: Uuid,
    ) -> Result<Report, anyhow::Error> {
        let report = self
            .repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("aduan tidak ditemukan"))?;

        if report.status.is_terminal() {
            return Err(anyhow::anyhow!(
                "aduan sudah ditindaklanjuti — tidak dapat diubah"
            ));
        }

        let new_status = if approved {
            ReportStatus::Resolved
        } else {
            ReportStatus::Rejected
        };

        self.repo
            .update_status(id, new_status, &action_note, reviewed_by)
            .await?
            .ok_or_else(|| anyhow::anyhow!("aduan tidak ditemukan"))
    }

    /// Endpoint terintegrasi "Terima & Suspend" (P9.1, Kelompok 6 Q9) — approve
    /// aduan SEKALIGUS suspend target dalam 1 aksi. Client dipilih berdasar
    /// `target_type`/`target_ad_type` yang TERSIMPAN di aduan (bukan input
    /// admin — mencegah spoofing target yang salah). Urutan operasi SENGAJA:
    /// suspend dulu, baru update status Report → Resolved (Hazard #4) — bila
    /// suspend gagal, Report TETAP di status semula (tidak ada state ambigu;
    /// bukan transaction lintas-service sungguhan, tapi ordering ini mencegah
    /// "aduan sudah Diterima tapi target belum ter-suspend").
    #[allow(clippy::too_many_arguments)]
    pub async fn admin_approve_and_suspend(
        &self,
        report_id: Uuid,
        input: super::dto::ApproveAndSuspendInput,
        admin_id: Uuid,
        auth_client: &dyn AuthClient,
        iklan_pekerjaan_client: Option<&dyn IklanPekerjaanClient>,
        iklan_pekerja_client: Option<&dyn IklanPekerjaClient>,
        iklan_barang_bekas_client: Option<&dyn IklanBarangBekasClient>,
    ) -> Result<Report, anyhow::Error> {
        let report = self
            .repo
            .find_by_id(report_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("aduan tidak ditemukan"))?;

        if report.status.is_terminal() {
            return Err(anyhow::anyhow!(
                "aduan sudah ditindaklanjuti — tidak dapat diubah"
            ));
        }

        let target_id = report
            .target_id
            .ok_or_else(|| anyhow::anyhow!("aduan tidak punya target — tidak dapat disuspend"))?;
        let is_permanent = input.is_permanent;

        match report.target_type {
            Some(ReportTargetType::User) => {
                if is_permanent {
                    auth_client
                        .suspend_permanently(target_id, &input.reason)
                        .await?;
                } else {
                    // UI web (pola sama SuspendIklanPayload/SuspendPenggunaPayload
                    // existing) tidak selalu kirim tanggal spesifik — fallback ke
                    // durasi default bila admin tidak memilih tanggal.
                    // ponytail: default 30 hari, naikkan ke date-picker eksplisit
                    // di web bila produk butuh kontrol durasi lebih presisi.
                    let days = input
                        .expires_at
                        .map(|dt| (dt - chrono::Utc::now()).num_days().max(1))
                        .unwrap_or(DEFAULT_TEMP_SUSPEND_DAYS);
                    auth_client
                        .suspend_temporarily(target_id, days, &input.reason)
                        .await?;
                }
            }
            Some(ReportTargetType::Iklan) => match report.target_ad_type {
                Some(ReportAdType::Pekerjaan) => {
                    iklan_pekerjaan_client
                        .ok_or_else(|| anyhow::anyhow!("iklan-pekerjaan-service tidak tersedia"))?
                        .suspend(
                            target_id,
                            is_permanent,
                            &input.reason,
                            input.expires_at,
                            admin_id,
                        )
                        .await?;
                }
                Some(ReportAdType::Pekerja) => {
                    iklan_pekerja_client
                        .ok_or_else(|| anyhow::anyhow!("iklan-pekerja-service tidak tersedia"))?
                        .suspend(
                            target_id,
                            is_permanent,
                            &input.reason,
                            input.expires_at,
                            admin_id,
                        )
                        .await?;
                }
                Some(ReportAdType::BarangBekas) => {
                    iklan_barang_bekas_client
                        .ok_or_else(|| {
                            anyhow::anyhow!("iklan-barang-bekas-service tidak tersedia")
                        })?
                        .suspend(
                            target_id,
                            is_permanent,
                            &input.reason,
                            input.expires_at,
                            admin_id,
                        )
                        .await?;
                }
                None => {
                    return Err(anyhow::anyhow!(
                        "jenis iklan (target_ad_type) tidak diketahui — tidak dapat menentukan service tujuan"
                    ));
                }
            },
            None => {
                return Err(anyhow::anyhow!(
                    "aduan tidak punya target_type — tidak dapat disuspend"
                ));
            }
        }

        self.repo
            .update_status(report_id, ReportStatus::Resolved, &input.reason, admin_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("aduan tidak ditemukan"))
    }

    // ── ReportClient in-process ───────────────────────────────────────────
    // Dipakai domain lain (cross-service) — arah pemakaian berbeda dari kedua
    // jalur mobile di atas, selalu punya target spesifik → `LaporkanIklan`.

    pub async fn create_report_inproc(
        &self,
        reporter_id: Uuid,
        target_type: report_service_client::ReportTargetType,
        target_id: Uuid,
        keterangan: String,
        evidence_object_key: Option<String>,
    ) -> Result<Uuid, report_service_client::ReportClientError> {
        self.create_internal(
            reporter_id,
            ReportType::LaporkanIklan,
            Some(target_type),
            None,
            Some(target_id),
            keterangan,
            evidence_object_key,
        )
        .await
        .map(|r| r.id)
        .map_err(|_| report_service_client::ReportClientError::Unavailable)
    }

    pub async fn get_status(
        &self,
        report_id: Uuid,
    ) -> Result<ReportStatus, report_service_client::ReportClientError> {
        self.repo
            .find_by_id(report_id)
            .await
            .map_err(|_| report_service_client::ReportClientError::Unavailable)?
            .map(|r| r.status)
            .ok_or(report_service_client::ReportClientError::NotFound)
    }

    // ── P1.3: enrichment demografis pelapor (batched, Hazard #5) ───────────

    async fn enrich_demographics(
        &self,
        reporter_ids: &[Uuid],
    ) -> HashMap<Uuid, ReporterDemographics> {
        let Some(user_client) = &self.user_client else {
            return HashMap::new();
        };
        let Ok(summaries) = user_client
            .get_demographic_summaries_by_auth_ids(reporter_ids)
            .await
        else {
            return HashMap::new();
        };

        // Dedup region id unik lintas seluruh halaman sebelum resolve nama (Hazard #5) —
        // pola sama `list_bider` (Kelompok 3 F-15), region-service belum sediakan batch lookup.
        let mut region_ids: HashSet<&str> = HashSet::new();
        for s in &summaries {
            for id in [&s.village_id, &s.district_id, &s.regency_id, &s.province_id]
                .into_iter()
                .flatten()
            {
                region_ids.insert(id.as_str());
            }
        }
        let mut region_names: HashMap<String, String> = HashMap::new();
        if let Some(region_client) = &self.region_client {
            for id in region_ids {
                if let Ok(region) = region_client.get_region(id).await {
                    region_names.insert(id.to_string(), region.name);
                }
            }
        }

        summaries
            .into_iter()
            .map(|s| {
                let demo = ReporterDemographics {
                    education_level: s.education_level,
                    gender: s.gender,
                    birth_date: s.birth_date,
                    address_line: s.address_line,
                    village_name: s
                        .village_id
                        .as_ref()
                        .and_then(|id| region_names.get(id))
                        .cloned(),
                    district_name: s
                        .district_id
                        .as_ref()
                        .and_then(|id| region_names.get(id))
                        .cloned(),
                    regency_name: s
                        .regency_id
                        .as_ref()
                        .and_then(|id| region_names.get(id))
                        .cloned(),
                    province_name: s
                        .province_id
                        .as_ref()
                        .and_then(|id| region_names.get(id))
                        .cloned(),
                    country: Some(s.country_code),
                };
                (s.auth_id, demo)
            })
            .collect()
    }
}

// ── Mapper functions ──────────────────────────────────────────────────────

pub fn to_report_resp(r: Report, demographics: Option<ReporterDemographics>) -> ReportResponse {
    let is_overdue = r.is_overdue();
    ReportResponse {
        id: r.id,
        reporter_id: r.reporter_id,
        report_type: r.report_type,
        target_type: r.target_type,
        target_id: r.target_id,
        keterangan: r.keterangan,
        evidence_object_key: r.evidence_object_key,
        status: r.status,
        action_note: r.action_note,
        reviewed_by: r.reviewed_by,
        due_date: r.due_date,
        is_overdue,
        created_at: r.created_at,
        updated_at: r.updated_at,
        reporter_demographics: demographics,
    }
}

pub fn to_detail_resp(r: Report, demographics: ReporterDemographics) -> ReportDetailResponse {
    let is_overdue = r.is_overdue();
    ReportDetailResponse {
        id: r.id,
        reporter_id: r.reporter_id,
        report_type: r.report_type,
        target_type: r.target_type,
        target_id: r.target_id,
        keterangan: r.keterangan,
        evidence_object_key: r.evidence_object_key,
        evidence_read_url: None, // diisi oleh handler setelah request presigned download
        status: r.status,
        action_note: r.action_note,
        reviewed_by: r.reviewed_by,
        due_date: r.due_date,
        is_overdue,
        created_at: r.created_at,
        updated_at: r.updated_at,
        reporter_demographics: demographics,
    }
}
