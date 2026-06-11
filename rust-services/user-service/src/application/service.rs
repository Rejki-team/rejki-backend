use std::sync::Arc;
use uuid::Uuid;

use auth_service_client::{AccountStatus, AuthClient};
use region_service_client::RegionClient;
use super::dto::{
    KycPersonalDataInput, KycSubmissionResponse, UpdateProfileInput, UserProfileResponse,
};
use crate::domain::entity::{KycSubmission, KycSubmissionStatus, UserProfile};
use crate::domain::repository::UserRepository;

pub struct UserService<R: UserRepository> {
    repo:         Arc<R>,
    auth_client:  Arc<dyn AuthClient>,
    region_client: Arc<dyn RegionClient>,
}

impl<R: UserRepository> UserService<R> {
    pub fn new(
        repo:          Arc<R>,
        auth_client:   Arc<dyn AuthClient>,
        region_client: Arc<dyn RegionClient>,
    ) -> Self {
        Self { repo, auth_client, region_client }
    }

    /// Profil sendiri — sertakan NIK ter-mask & status KYC.
    pub async fn get_profile(&self, user_id: Uuid) -> Result<UserProfileResponse, anyhow::Error> {
        let profile = self
            .repo
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("profil tidak ditemukan"))?;
        let kyc = self.repo.get_latest_submission(profile.id).await?;
        Ok(self.to_response(&profile, kyc.as_ref()))
    }

    /// Cari profil by auth_id (dipakai get_me handler).
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

    pub async fn update_profile(
        &self,
        user_id: Uuid,
        input: UpdateProfileInput,
    ) -> Result<UserProfileResponse, anyhow::Error> {
        let profile = self
            .repo
            .update(
                user_id,
                input.full_name.as_deref(),
                input.avatar.as_deref(),
                input.bio.as_deref(),
                input.phone.as_deref(),
            )
            .await?;
        let kyc = self.repo.get_latest_submission(profile.id).await?;
        Ok(self.to_response(&profile, kyc.as_ref()))
    }

    pub async fn update_avatar(&self, id: Uuid, object_key: &str) -> Result<(), anyhow::Error> {
        self.repo.update_avatar(id, object_key).await
    }

    /// Kirim data diri KYC — validasi rantai wilayah, enkripsi NIK, simpan, buat submission,
    /// lalu panggil AuthClient untuk transisi status `profile_incomplete → pending_kyc`.
    pub async fn submit_kyc(
        &self,
        user_id: Uuid,
        input: KycPersonalDataInput,
    ) -> Result<KycSubmissionResponse, anyhow::Error> {
        // Validasi rantai wilayah via RegionClient (keputusan K13).
        let valid = self
            .region_client
            .validate_chain(&input.province_id, &input.regency_id, &input.district_id, &input.village_id)
            .await
            .map_err(|e| anyhow::anyhow!("gagal validasi wilayah: {e}"))?;
        if !valid {
            return Err(anyhow::anyhow!("rantai wilayah tidak konsisten"));
        }

        // NIK immutable check — jika sudah terisi, tolak perubahan (K5).
        if let Some(profile) = self.repo.find_by_id(user_id).await? {
            if profile.nik_encrypted.is_some() {
                return Err(anyhow::anyhow!("NIK tidak dapat diubah"));
            }
        }

        // Enkripsi NIK via common-crypto (AES-256-GCM, K14). Gagal → batal simpan.
        let nik_last4 = input.nik[input.nik.len().saturating_sub(4)..].to_owned();
        let nik_encrypted = common_crypto::encrypt(&input.nik)
            .map_err(|e| anyhow::anyhow!("gagal enkripsi NIK: {e}"))?;

        // Simpan data diri + buat submission
        let mut profile = self.repo.find_by_id(user_id).await?
            .ok_or_else(|| anyhow::anyhow!("profil tidak ditemukan"))?;
        profile.full_name       = Some(input.full_name);
        profile.education_level = Some(input.education_level);
        profile.gender          = Some(input.gender);
        profile.birth_date      = Some(input.birth_date);
        profile.address_line    = Some(input.address_line);
        profile.country_code    = input.country_code;
        profile.province_id     = Some(input.province_id);
        profile.regency_id      = Some(input.regency_id);
        profile.district_id     = Some(input.district_id);
        profile.village_id      = Some(input.village_id);
        profile.nik_encrypted   = Some(nik_encrypted.into_bytes());
        profile.nik_last4       = Some(nik_last4);
        self.repo.update_profile(&profile).await?;

        let submission = self.repo.create_submission(profile.id).await?;

        // Transisi status akun via AuthClient (K2: in-process).
        let _ = self
            .auth_client
            .set_account_status(profile.auth_id, AccountStatus::PendingKyc)
            .await;

        Ok(KycSubmissionResponse {
            id:          submission.id,
            status:      submission.status.as_str().into(),
            review_note: None,
            reviewed_at: None,
            created_at:  submission.created_at.to_rfc3339(),
        })
    }

    pub async fn get_kyc_status(&self, user_id: Uuid) -> Result<Option<KycSubmissionResponse>, anyhow::Error> {
        let profile = self.repo.find_by_id(user_id).await?
            .ok_or_else(|| anyhow::anyhow!("profil tidak ditemukan"))?;
        let sub = self.repo.get_latest_submission(profile.id).await?;
        Ok(sub.map(|s| KycSubmissionResponse {
            id:          s.id,
            status:      s.status.as_str().into(),
            review_note: s.review_note,
            reviewed_at: s.reviewed_at.map(|d| d.to_rfc3339()),
            created_at:  s.created_at.to_rfc3339(),
        }))
    }

    pub async fn review_kyc(
        &self,
        submission_id: Uuid,
        admin_id:      Uuid,
        approved:      bool,
        note:          Option<&str>,
    ) -> Result<(), anyhow::Error> {
        let status = if approved {
            KycSubmissionStatus::Approved
        } else {
            KycSubmissionStatus::Rejected
        };

        // Dapatkan submission untuk mencari profile_id → auth_id.
        let submission = self
            .repo
            .get_submission_by_id(submission_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("submission tidak ditemukan"))?;

        self.repo
            .review_submission(submission_id, status, admin_id, note)
            .await?;

        // Transisi status akun via AuthClient (K2): submission → profile → auth_id.
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

        Ok(())
    }

    fn to_response(
        &self,
        p: &UserProfile,
        kyc: Option<&KycSubmission>,
    ) -> UserProfileResponse {
        let nik_masked = p.nik_last4.as_ref().map(|l4| format!("xxx...{l4}"));
        let kyc_status = kyc.map(|s| s.status.as_str().to_owned());
        UserProfileResponse {
            id:         p.id,
            username:   p.username.clone(),
            full_name:  p.full_name.clone(),
            avatar:     p.avatar.clone(),
            bio:        p.bio.clone(),
            phone:      p.phone.clone(),
            nik_masked,
            kyc_status,
        }
    }
}
