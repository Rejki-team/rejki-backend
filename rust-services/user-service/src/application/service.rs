use std::sync::Arc;

use uuid::Uuid;

use auth_service_client::{AccountStatus, AuthClient};
use notification_service_client::{EmailMessage, NotificationClient, NotificationPayload};
use region_service_client::RegionClient;
use storage_service_client::StorageClient;

use super::dto::{
    AdminKycDetail, AdminKycListItem, AdminKycListQuery, CommitDocumentInput, KycPersonalDataInput,
    KycSubmissionResponse, UpdateProfileInput, UserProfileResponse,
};
use crate::domain::entity::{
    DocumentAccessAction, KycSubmission, KycSubmissionStatus, RekeningInfo, ReviewError,
    UserProfile,
};
use crate::domain::repository::{
    AdminKycListParams, AdminKycRow, UpdateProfileParams, UserRepository,
};

/// Cooldown KYC setelah rejection — 3 hari kalender (M1, rename dari BUSINESS_DAYS).
const KYC_COOLDOWN_DAYS: i64 = 3;
/// Batas baris default untuk listing admin bila klien tak mengirim `limit`.
const ADMIN_LIST_DEFAULT_LIMIT: i64 = 20;
/// Batas atas baris untuk ekspor CSV (hindari memori tak terbatas / Memory Safe).
const ADMIN_CSV_MAX: i64 = 10_000;
/// Jenis dokumen KYC yang sah.
const DOCUMENT_KINDS: [&str; 2] = ["ktp", "selfie"];

/// Hasil listing admin yang siap dirender handler (items + meta paginasi).
pub struct AdminKycListPage {
    pub items: Vec<AdminKycListItem>,
    pub total: i64,
    pub page: u32,
    pub per_page: u32,
}

pub struct UserService<R: UserRepository> {
    repo: Arc<R>,
    auth_client: Arc<dyn AuthClient>,
    region_client: Arc<dyn RegionClient>,
    storage_client: Option<Arc<dyn StorageClient>>,
    notifier: Option<Arc<dyn NotificationClient>>,
}

impl<R: UserRepository> UserService<R> {
    pub fn new(
        repo: Arc<R>,
        auth_client: Arc<dyn AuthClient>,
        region_client: Arc<dyn RegionClient>,
        storage_client: Option<Arc<dyn StorageClient>>,
        notifier: Option<Arc<dyn NotificationClient>>,
    ) -> Self {
        Self {
            repo,
            auth_client,
            region_client,
            storage_client,
            notifier,
        }
    }

    pub async fn get_profile(&self, user_id: Uuid) -> Result<UserProfileResponse, anyhow::Error> {
        let profile = self
            .repo
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("profil tidak ditemukan"))?;
        let kyc = self.repo.get_latest_submission(profile.id).await?;
        Ok(self.to_response(&profile, kyc.as_ref()))
    }

    pub async fn get_profile_by_auth_id(
        &self,
        auth_id: Uuid,
    ) -> Result<UserProfileResponse, anyhow::Error> {
        let profile = self
            .repo
            .find_by_auth_id(auth_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("profil tidak ditemukan"))?;
        let kyc = self.repo.get_latest_submission(profile.id).await?;
        Ok(self.to_response(&profile, kyc.as_ref()))
    }

    pub async fn resolve_profile_id(&self, auth_id: Uuid) -> Result<Uuid, anyhow::Error> {
        let profile = self
            .repo
            .find_by_auth_id(auth_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("profil tidak ditemukan"))?;
        Ok(profile.id)
    }

    pub async fn update_profile(
        &self,
        profile_id: Uuid,
        input: UpdateProfileInput,
    ) -> Result<UserProfileResponse, anyhow::Error> {
        let profile = self
            .repo
            .update(UpdateProfileParams {
                id: profile_id,
                full_name: input.full_name,
                avatar: input.avatar,
                bio: input.bio,
                phone: input.phone,
                rekening: input.rekening,
            })
            .await?;
        let kyc = self.repo.get_latest_submission(profile.id).await?;
        Ok(self.to_response(&profile, kyc.as_ref()))
    }

    pub async fn update_avatar(
        &self,
        profile_id: Uuid,
        object_key: &str,
    ) -> Result<(), anyhow::Error> {
        self.repo.update_avatar(profile_id, object_key).await
    }

    pub async fn submit_kyc(
        &self,
        auth_id: Uuid,
        email: Option<&str>,
        input: KycPersonalDataInput,
    ) -> Result<KycSubmissionResponse, anyhow::Error> {
        // Validasi digit-only NIK (H1) — lapis tambahan selain validator derive.
        if !input.nik.chars().all(|c| c.is_ascii_digit()) {
            return Err(anyhow::anyhow!("NIK harus berupa 16 digit angka"));
        }

        let valid = self
            .region_client
            .validate_chain(
                &input.province_id,
                &input.regency_id,
                &input.district_id,
                &input.village_id,
            )
            .await
            .map_err(|e| anyhow::anyhow!("gagal validasi wilayah: {e}"))?;
        if !valid {
            return Err(anyhow::anyhow!("rantai wilayah tidak konsisten"));
        }

        // ── Transaction boundary (C2): atomic multi-step write ──────────────
        // find_by_auth_id + update_profile + create_submission dalam satu TX.
        // Jika create_submission gagal → rollback (NIK tidak tersimpan).
        let mut tx = self.repo.begin_transaction().await?;

        let mut profile = tx
            .find_by_auth_id(auth_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("profil tidak ditemukan"))?;

        // NIK immutable (K5): cek pada record yang baru di-fetch dalam TX.
        if profile.nik_encrypted.is_some() {
            return Err(anyhow::anyhow!("NIK tidak dapat diubah"));
        }

        // Cooldown 3 hari setelah rejected (K16).
        // Baca di luar TX — read-only, isolasi snapshot cukup.
        if let Some(last_sub) = self.repo.get_latest_submission(profile.id).await? {
            if last_sub.status == KycSubmissionStatus::Rejected {
                if let Some(reviewed_at) = last_sub.reviewed_at {
                    let cooldown_end = reviewed_at + chrono::Duration::days(KYC_COOLDOWN_DAYS);
                    if chrono::Utc::now() < cooldown_end {
                        return Err(anyhow::anyhow!(
                            "mohon tunggu hingga {cooldown_end} sebelum mengirim ulang KYC"
                        ));
                    }
                }
            }
        }

        let nik_last4 = input.nik[input.nik.len().saturating_sub(4)..].to_owned();
        let nik_encrypted = common_crypto::encrypt(&input.nik)
            .map_err(|e| anyhow::anyhow!("gagal enkripsi NIK: {e}"))?;

        profile.full_name = Some(input.full_name);
        profile.education_level = Some(input.education_level);
        profile.gender = Some(input.gender);
        profile.birth_date = Some(input.birth_date);
        profile.address_line = Some(input.address_line);
        profile.country_code = input.country_code;
        profile.province_id = Some(input.province_id);
        profile.regency_id = Some(input.regency_id);
        profile.district_id = Some(input.district_id);
        profile.village_id = Some(input.village_id);
        profile.nik_encrypted = Some(nik_encrypted.into_bytes());
        profile.nik_last4 = Some(nik_last4);

        // Atomic write: guard `AND nik_encrypted IS NULL` — cek rows_affected (C1).
        let profile_updated = tx.update_profile(&profile).await?;
        if !profile_updated {
            // NIK sudah ada (concurrent request lebih dulu) — rollback + error.
            let _ = tx.rollback().await;
            return Err(anyhow::anyhow!("NIK tidak dapat diubah"));
        }

        let submission = tx.create_submission(profile.id).await?;
        tx.commit().await?;
        // ── End transaction boundary ─────────────────────────────────────────

        // Transisi status akun (K2) — propagasi error, best-effort setelah commit.
        self.auth_client
            .set_account_status(profile.auth_id, AccountStatus::PendingKyc)
            .await
            .map_err(|e| anyhow::anyhow!("gagal transisi status akun: {e}"))?;

        // Notifikasi KYC submission (K10/K12): push + in-app + email saat awal.
        self.notify(
            profile.auth_id,
            "KYC Dikirim",
            "Data KYC Anda telah dikirim dan sedang menunggu verifikasi.",
        )
        .await;
        if let Some(to) = email {
            self.notify_email(
                to,
                "Verifikasi KYC Rejki — Sedang Diproses",
                "Data KYC Anda telah diterima dan sedang dalam antrian verifikasi. ",
            )
            .await;
        }

        Ok(KycSubmissionResponse {
            id: submission.id,
            status: submission.status.as_str().into(),
            review_note: None,
            reviewed_at: None,
            created_at: submission.created_at.to_rfc3339(),
        })
    }

    pub async fn get_kyc_status(
        &self,
        auth_id: Uuid,
    ) -> Result<Option<KycSubmissionResponse>, anyhow::Error> {
        let profile = self
            .repo
            .find_by_auth_id(auth_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("profil tidak ditemukan"))?;
        let sub = self.repo.get_latest_submission(profile.id).await?;
        Ok(sub.map(|s| KycSubmissionResponse {
            id: s.id,
            status: s.status.as_str().into(),
            review_note: s.review_note,
            reviewed_at: s.reviewed_at.map(|d| d.to_rfc3339()),
            created_at: s.created_at.to_rfc3339(),
        }))
    }

    /// Tinjau pengajuan KYC (approve/reject).
    /// Idempoten (D5): pengajuan berstatus terminal ditolak via `ReviewError::AlreadyReviewed`.
    /// Atomik TOCTOU-safe: `review_submission` melakukan `UPDATE ... WHERE status='pending'`
    /// dan mengembalikan false bila baris sudah terminal (race-condition proof).
    /// Reject memicu auto-purge dokumen (D4), best-effort + logged.
    pub async fn review_kyc(
        &self,
        submission_id: Uuid,
        admin_id: Uuid,
        approved: bool,
        note: Option<&str>,
    ) -> Result<(), ReviewError> {
        let status = if approved {
            KycSubmissionStatus::Approved
        } else {
            KycSubmissionStatus::Rejected
        };

        // Ambil submission untuk cek keberadaan + dapatkan profile_id.
        let submission = self
            .repo
            .get_submission_by_id(submission_id)
            .await
            .map_err(ReviewError::Other)?
            .ok_or(ReviewError::NotFound)?;

        // Atomik: UPDATE hanya bila status saat ini 'pending'. Kalau tidak ada
        // baris yang terpengaruh → submission sudah terminal → AlreadyReviewed.
        let updated = self
            .repo
            .review_submission(submission_id, status, admin_id, note)
            .await
            .map_err(ReviewError::Other)?;

        if !updated {
            return Err(ReviewError::AlreadyReviewed);
        }

        let profile = self
            .repo
            .find_by_id(submission.profile_id)
            .await
            .map_err(ReviewError::Other)?
            .ok_or_else(|| {
                ReviewError::Other(anyhow::anyhow!("profil tidak ditemukan untuk submission"))
            })?;

        let target_status = if approved {
            AccountStatus::Active
        } else {
            AccountStatus::Rejected
        };

        self.auth_client
            .set_account_status(profile.auth_id, target_status)
            .await
            .map_err(|e| anyhow::anyhow!("gagal mengubah status akun: {e}"))?;

        // Notifikasi status KYC (K10/K12): push + in-app + email pada hasil akhir.
        // Gunakan String untuk menghindari Box::leak pada branch rejected (format!).
        let (push_title, push_body, email_subject, email_body): (String, String, String, String) =
            if approved {
                (
                    "KYC Disetujui".into(),
                    "Selamat! KYC Anda telah disetujui. Anda kini dapat menggunakan semua fitur."
                        .into(),
                    "KYC Rejki Disetujui \u{2705}".into(),
                    "Selamat! Verifikasi KYC Anda telah disetujui. Akun Anda kini aktif penuh."
                        .into(),
                )
            } else {
                let reason = note.unwrap_or("Tidak ada keterangan");
                let body_text = format!(
                    "KYC Anda ditolak: {reason}. Anda dapat mengirim ulang setelah 3 hari kerja."
                );
                let email_body_text = format!(
                "KYC Anda ditolak dengan alasan: {reason}\n\nAnda dapat memperbaiki data dan mengirim ulang setelah 3 hari kerja."
            );
                (
                    "KYC Ditolak".into(),
                    body_text,
                    "KYC Rejki Ditolak".into(),
                    email_body_text,
                )
            };

        self.notify(profile.auth_id, &push_title, &push_body).await;

        // Email ke user yang direview (bukan admin) — K12 wajib email hasil akhir.
        match self.auth_client.get_account_email(profile.auth_id).await {
            Ok(to) => {
                self.notify_email(&to, &email_subject, &email_body).await;
            }
            Err(e) => {
                tracing::warn!(
                    user_id = %profile.auth_id,
                    error = %e,
                    "tidak dapat mengambil email untuk notifikasi hasil KYC"
                );
            }
        }

        // Auto-purge dokumen saat penolakan (D4, FR-ADM-USR-05, UU PDP).
        // Best-effort + logged: kegagalan storage TIDAK membatalkan keputusan reject
        // yang sudah tercatat; sisa objek dicatat untuk pembersihan lanjutan.
        if !approved {
            if let Err(e) = self.purge_documents(profile.id).await {
                tracing::warn!(
                    profile_id = %profile.id,
                    submission_id = %submission_id,
                    error = %e,
                    "auto-purge dokumen gagal saat penolakan KYC — perlu pembersihan lanjutan"
                );
            }
        }

        Ok(())
    }

    /// Commit dokumen KYC: verifikasi magic bytes, validasi ownership, simpan object key.
    /// Alur dua-langkah (design.md D6): request → upload ke storage → commit.
    pub async fn commit_document(
        &self,
        auth_id: Uuid,
        request_id: Option<&str>,
        input: CommitDocumentInput,
    ) -> Result<(), anyhow::Error> {
        // Decode base64 header & verifikasi magic bytes (2.3).
        let head = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            &input.magic_head_b64,
        )
        .map_err(|e| anyhow::anyhow!("magic_head_b64 tidak valid base64: {e}"))?;
        storage_service_client::verify_magic_bytes(&input.mime, &head)
            .map_err(|e| anyhow::anyhow!("verifikasi berkas gagal: {e}"))?;

        // Validasi object_key berprefiks uploads/{kind}/{auth_id}/ — anti-IDOR.
        let expected_prefix = format!("uploads/{}/{}", input.kind, auth_id);
        if !input.object_key.starts_with(&expected_prefix) {
            return Err(anyhow::anyhow!("object_key tidak sesuai dengan pemilik"));
        }

        // Resolve profile & dapatkan submission pending terbaru.
        let profile = self
            .repo
            .find_by_auth_id(auth_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("profil tidak ditemukan"))?;

        let submission = self
            .repo
            .get_latest_submission(profile.id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("tidak ada submission KYC untuk dicommit"))?;

        self.repo
            .set_document_key(submission.id, &input.kind, &input.object_key)
            .await?;

        // Audit: commit tercatat (Q2).
        self.repo
            .log_document_access(
                auth_id,
                &input.object_key,
                DocumentAccessAction::Commit,
                request_id,
            )
            .await?;

        Ok(())
    }

    /// Dapatkan presigned URL baca sementara untuk dokumen KYC milik sendiri.
    /// Kembalikan `None` bila dokumen belum diupload.
    /// Owner-only; akses admin → `admin_get_document_url` (teraudit).
    pub async fn get_document_url(
        &self,
        auth_id: Uuid,
        kind: &str,
        request_id: Option<&str>,
    ) -> Result<Option<String>, anyhow::Error> {
        let profile = self
            .repo
            .find_by_auth_id(auth_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("profil tidak ditemukan"))?;

        let submission = match self.repo.get_latest_submission(profile.id).await? {
            Some(s) => s,
            None => return Ok(None),
        };

        let object_key = match kind {
            "ktp" => &submission.ktp_object_key,
            "selfie" => &submission.selfie_object_key,
            _ => return Err(anyhow::anyhow!("jenis dokumen tidak dikenal: {kind}")),
        };

        let key = match object_key {
            Some(k) => k,
            None => return Ok(None),
        };

        // Audit: read_issued tercatat (Q2).
        self.repo
            .log_document_access(auth_id, key, DocumentAccessAction::ReadIssued, request_id)
            .await?;

        let url = match self.storage_client.as_ref() {
            Some(storage) => storage
                .request_download(key)
                .await
                .map_err(|e| anyhow::anyhow!("gagal membuat presigned download URL: {e}"))?,
            None => return Err(anyhow::anyhow!("storage tidak tersedia")),
        };

        Ok(Some(url))
    }

    /// Catat audit `upload_issued` saat presigned upload URL diterbitkan (Q2).
    pub async fn log_upload_issued(
        &self,
        actor_id: Uuid,
        object_key: &str,
        request_id: Option<&str>,
    ) -> Result<(), anyhow::Error> {
        self.repo
            .log_document_access(
                actor_id,
                object_key,
                DocumentAccessAction::UploadIssued,
                request_id,
            )
            .await
    }

    /// Hapus dokumen KYC dari storage & kosongkan referensi di submission.
    /// Pemicu: penolakan KYC (auto, lihat `review_kyc`). Idempoten: aman dipanggil
    /// ulang (early-return bila tak ada submission/dokumen).
    pub async fn purge_documents(&self, profile_id: Uuid) -> Result<(), anyhow::Error> {
        let submission = match self.repo.get_latest_submission(profile_id).await? {
            Some(s) => s,
            None => return Ok(()), // tidak ada dokumen untuk dihapus
        };

        // Hapus dari storage.
        if let Some(ref storage) = self.storage_client {
            if let Some(ref key) = submission.ktp_object_key {
                if let Err(e) = storage.delete(key).await {
                    tracing::warn!(object_key = %key, error = %e, "gagal hapus ktp_object_key");
                }
            }
            if let Some(ref key) = submission.selfie_object_key {
                if let Err(e) = storage.delete(key).await {
                    tracing::warn!(object_key = %key, error = %e, "gagal hapus selfie_object_key");
                }
            }
        }

        self.repo.clear_document_keys(submission.id).await?;

        tracing::info!(
            profile_id = %profile_id,
            submission_id = %submission.id,
            "dokumen KYC dimusnahkan"
        );
        Ok(())
    }

    /// Resolve profile_id dari auth user_id, lalu musnahkan dokumen KYC.
    /// Dipanggil oleh UserInProcessClient saat suspend permanen (D4).
    pub async fn purge_kyc_by_user_id(&self, user_id: Uuid) -> Result<(), anyhow::Error> {
        let profile_id = self.resolve_profile_id(user_id).await?;
        self.purge_documents(profile_id).await
    }

    // ── Admin: listing, detail, & akses dokumen (add-user-admin-management) ──────

    /// Daftar pengajuan KYC untuk admin (filter/sort/pagination). NIK ter-mask.
    pub async fn admin_list_submissions(
        &self,
        query: AdminKycListQuery,
    ) -> Result<AdminKycListPage, anyhow::Error> {
        let limit = normalize_limit(query.limit);
        let offset = query.offset.unwrap_or(0).max(0);
        let result = self
            .repo
            .admin_list_submissions(AdminKycListParams {
                q: query.q,
                status: query.status,
                sort_dir: query.sort_dir,
                limit,
                offset,
            })
            .await?;

        let per_page = limit as u32;
        let page = (offset / limit) as u32 + 1;
        Ok(AdminKycListPage {
            items: result.items.iter().map(to_admin_list_item).collect(),
            total: result.total,
            page,
            per_page,
        })
    }

    /// Ambil seluruh baris sesuai filter aktif untuk ekspor CSV (dibatasi ADMIN_CSV_MAX).
    pub async fn admin_export_submissions(
        &self,
        query: AdminKycListQuery,
    ) -> Result<Vec<AdminKycListItem>, anyhow::Error> {
        let rows = self
            .repo
            .admin_list_submissions_all(AdminKycListParams {
                q: query.q,
                status: query.status,
                sort_dir: query.sort_dir,
                limit: ADMIN_CSV_MAX,
                offset: 0,
            })
            .await?;
        Ok(rows.iter().map(to_admin_list_item).collect())
    }

    /// Detail satu pengajuan KYC untuk pop-up admin (NIK ter-mask). IDOR→None→404.
    pub async fn admin_get_submission(
        &self,
        submission_id: Uuid,
    ) -> Result<Option<AdminKycDetail>, anyhow::Error> {
        Ok(self
            .repo
            .get_submission_with_profile(submission_id)
            .await?
            .as_ref()
            .map(to_admin_detail))
    }

    /// Terbitkan presigned read URL untuk dokumen KYC milik pengajuan tertentu (admin).
    /// - `kind` ∈ {ktp, selfie} (else Err → 422).
    /// - submission di-resolve by id (bukan claims) — akses lintas-pengguna teraudit.
    /// - `Ok(None)` bila dokumen sudah dimusnahkan / belum ada (handler → tidak-tersedia).
    /// - audit `read_issued` dengan aktor = admin dicatat SEBELUM URL terbit (D3).
    pub async fn admin_get_document_url(
        &self,
        submission_id: Uuid,
        kind: &str,
        admin_id: Uuid,
        request_id: Option<&str>,
    ) -> Result<Option<String>, anyhow::Error> {
        if !DOCUMENT_KINDS.contains(&kind) {
            return Err(anyhow::anyhow!("jenis dokumen tidak dikenal: {kind}"));
        }

        let submission = match self.repo.get_submission_with_profile(submission_id).await? {
            Some(s) => s,
            None => return Ok(None),
        };

        let object_key = match kind {
            "ktp" => submission.ktp_object_key,
            "selfie" => submission.selfie_object_key,
            // Sudah divalidasi di atas; cabang ini tak tercapai.
            _ => return Err(anyhow::anyhow!("jenis dokumen tidak dikenal: {kind}")),
        };

        let key = match object_key {
            Some(k) => k,
            None => return Ok(None), // dokumen sudah dimusnahkan / belum diupload
        };

        // Audit akses oleh admin (akuntabilitas) — dicatat sebelum URL diterbitkan.
        self.repo
            .log_document_access(admin_id, &key, DocumentAccessAction::ReadIssued, request_id)
            .await?;

        let url = match self.storage_client.as_ref() {
            Some(storage) => storage
                .request_download(&key)
                .await
                .map_err(|e| anyhow::anyhow!("gagal membuat presigned download URL: {e}"))?,
            None => return Err(anyhow::anyhow!("storage tidak tersedia")),
        };

        Ok(Some(url))
    }

    // ── helpers ──────────────────────────────────────────────────────────────────

    async fn notify(&self, user_id: Uuid, title: &str, body: &str) {
        if let Some(ref notifier) = self.notifier {
            let payload = NotificationPayload {
                title: title.to_owned(),
                body: body.to_owned(),
                data: None,
            };
            if let Err(e) = notifier.send(user_id, payload).await {
                tracing::warn!(user_id = %user_id, error = %e, "gagal kirim notifikasi KYC");
            }
        }
    }

    async fn notify_email(&self, to: &str, subject: &str, body: &str) {
        if let Some(ref notifier) = self.notifier {
            let msg = EmailMessage {
                to: to.to_owned(),
                subject: subject.to_owned(),
                body: body.to_owned(),
            };
            if let Err(e) = notifier.send_email(msg).await {
                tracing::warn!(to = %to, error = %e, "gagal kirim email KYC");
            }
        }
    }

    fn to_response(&self, p: &UserProfile, kyc: Option<&KycSubmission>) -> UserProfileResponse {
        let nik_masked = p.nik_last4.as_ref().map(|l4| format!("xxx...{l4}"));
        let kyc_status = kyc.map(|s| s.status.as_str().to_owned());

        // Parse rekening dari ciphertext — decrypt + deserialise
        let (rekening_bank, rekening_masked, rekening_holder_masked) = p
            .rekening_encrypted
            .as_deref()
            .and_then(|enc| common_crypto::decrypt(enc).ok())
            .and_then(|json| serde_json::from_str::<RekeningInfo>(&json).ok())
            .map(|rek| {
                let bank = rek.bank.clone();
                let num = mask_rekening_number(&rek.number);
                let holder = mask_holder(&rek.holder);
                (Some(bank), Some(num), Some(holder))
            })
            .unwrap_or((None, None, None));

        UserProfileResponse {
            id: p.id,
            username: p.username.clone(),
            full_name: p.full_name.clone(),
            avatar: p.avatar.clone(),
            bio: p.bio.clone(),
            phone: p.phone.clone(),
            // role di-resolve oleh handler dari AuthClaims; di sini diset None dulu.
            role: None,
            nik_masked,
            kyc_status,
            rekening_bank,
            rekening_masked,
            rekening_holder_masked,
        }
    }
}

// ── Free helpers (mapping & normalisasi) ───────────────────────────────────────

/// Mask NIK menjadi `xxx...1234` dari 4 digit terakhir; None bila belum ada.
fn mask_nik(nik_last4: Option<&str>) -> Option<String> {
    nik_last4.map(|l4| format!("xxx...{l4}"))
}

/// Mask nomor rekening — tampilkan 4 digit terakhir, sisanya ****.
fn mask_rekening_number(num: &str) -> String {
    if num.len() >= 4 {
        format!("****{}", &num[num.len() - 4..])
    } else {
        "****".to_string()
    }
}

/// Mask nama pemilik rekening — karakter pertama dan terakhir, tengah ***.
/// Contoh: "John Doe" → "J***e", "A" → "A***"
fn mask_holder(name: &str) -> String {
    if name.len() <= 2 {
        if name.is_empty() {
            "***".to_string()
        } else {
            format!("{}***", &name[..1])
        }
    } else {
        format!("{}***{}", &name[..1], &name[name.len() - 1..])
    }
}

/// Normalisasi `limit` ke rentang aman [1, ADMIN_CSV_MAX]; default bila None.
fn normalize_limit(limit: Option<i64>) -> i64 {
    limit
        .unwrap_or(ADMIN_LIST_DEFAULT_LIMIT)
        .clamp(1, ADMIN_CSV_MAX)
}

/// Map baris repo → item listing admin (NIK ter-mask, tanpa object key).
fn to_admin_list_item(r: &AdminKycRow) -> AdminKycListItem {
    AdminKycListItem {
        id: r.submission_id,
        full_name: r.full_name.clone(),
        education_level: r.education_level.clone(),
        gender: r.gender.clone(),
        birth_date: r.birth_date,
        address_line: r.address_line.clone(),
        country_code: r.country_code.clone(),
        province_id: r.province_id.clone(),
        regency_id: r.regency_id.clone(),
        district_id: r.district_id.clone(),
        village_id: r.village_id.clone(),
        nik_masked: mask_nik(r.nik_last4.as_deref()),
        status: r.status.as_str().to_owned(),
        created_at: r.created_at.to_rfc3339(),
    }
}

/// Map baris repo → detail admin (NIK ter-mask + penanda ketersediaan dokumen).
fn to_admin_detail(r: &AdminKycRow) -> AdminKycDetail {
    AdminKycDetail {
        id: r.submission_id,
        profile_id: r.profile_id,
        full_name: r.full_name.clone(),
        education_level: r.education_level.clone(),
        gender: r.gender.clone(),
        birth_date: r.birth_date,
        address_line: r.address_line.clone(),
        country_code: r.country_code.clone(),
        province_id: r.province_id.clone(),
        regency_id: r.regency_id.clone(),
        district_id: r.district_id.clone(),
        village_id: r.village_id.clone(),
        nik_masked: mask_nik(r.nik_last4.as_deref()),
        status: r.status.as_str().to_owned(),
        has_ktp: r.ktp_object_key.is_some(),
        has_selfie: r.selfie_object_key.is_some(),
        created_at: r.created_at.to_rfc3339(),
    }
}
