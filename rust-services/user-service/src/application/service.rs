use std::sync::Arc;

use uuid::Uuid;

use auth_service_client::{AccountStatus, AuthClient};
use notification_service_client::{EmailMessage, NotificationClient, NotificationPayload};
use region_service_client::RegionClient;
use storage_service_client::StorageClient;

use super::dto::{
    CommitDocumentInput, KycPersonalDataInput, KycSubmissionResponse, UpdateProfileInput,
    UserProfileResponse,
};
use crate::domain::entity::{
    DocumentAccessAction, KycSubmission, KycSubmissionStatus, UserProfile,
};
use crate::domain::repository::UserRepository;

const KYC_COOLDOWN_BUSINESS_DAYS: i64 = 3;

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
            .update(
                profile_id,
                input.full_name.as_deref(),
                input.avatar.as_deref(),
                input.bio.as_deref(),
                input.phone.as_deref(),
            )
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

        // Satu kali fetch — eliminasi TOCTOU race condition (C1).
        let mut profile = self
            .repo
            .find_by_auth_id(auth_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("profil tidak ditemukan"))?;

        // NIK immutable (K5): cek pada record yang baru di-fetch.
        if profile.nik_encrypted.is_some() {
            return Err(anyhow::anyhow!("NIK tidak dapat diubah"));
        }

        // Cooldown 3 hari kerja setelah rejected (K16).
        if let Some(last_sub) = self.repo.get_latest_submission(profile.id).await? {
            if last_sub.status == KycSubmissionStatus::Rejected {
                if let Some(reviewed_at) = last_sub.reviewed_at {
                    let cooldown_end =
                        reviewed_at + chrono::Duration::days(KYC_COOLDOWN_BUSINESS_DAYS);
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
        self.repo.update_profile(&profile).await?;

        let submission = self.repo.create_submission(profile.id).await?;

        // Transisi status akun (K2) — propagasi error, bukan silent discard (C2).
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

    pub async fn review_kyc(
        &self,
        submission_id: Uuid,
        admin_id: Uuid,
        approved: bool,
        note: Option<&str>,
    ) -> Result<(), anyhow::Error> {
        let status = if approved {
            KycSubmissionStatus::Approved
        } else {
            KycSubmissionStatus::Rejected
        };

        let submission = self
            .repo
            .get_submission_by_id(submission_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("submission tidak ditemukan"))?;

        self.repo
            .review_submission(submission_id, status, admin_id, note)
            .await?;

        let profile = self
            .repo
            .find_by_id(submission.profile_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("profil tidak ditemukan untuk submission"))?;

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
    /// Owner-only; admin read = TODO RBAC.
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
    /// Pemicu: penutupan akun (milik auth-service — belum didefinisikan).
    /// Task 6.4 parsial: method tersedia tapi tanpa HTTP trigger auto.
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
        }
    }
}
