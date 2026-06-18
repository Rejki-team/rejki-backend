//! Definisi OpenAPI untuk Swagger UI — HANYA disajikan saat `APP_ENV=development`
//! (lihat gate di `main.rs`). Dokumen ini sengaja minimal dan tumbuh inkremental.
//!
//! Catatan arsitektur (design D7): skema untuk dokumentasi didefinisikan sebagai
//! "mirror" lokal di composition root ini, BUKAN dengan menambah `utoipa::ToSchema`
//! ke DTO crate domain (mis. `auth-service`). Dengan begitu crate domain tidak
//! menarik dependency dokumentasi hanya demi Swagger.

use serde::{Deserialize, Serialize};
use utoipa::{OpenApi, ToSchema};

// Mirror DTO untuk dokumentasi — lihat design D7 (composition root mirror pattern).
// Semua tipe di sini adalah replika dari *-service crate, BUKAN import langsung.
// Crate domain tidak menarik dependensi dokumentasi hanya demi Swagger.

// ── Auth ──────────────────────────────────────────────────────────────────────

/// Payload registrasi (`POST /api/v1/auth/register`).
#[allow(dead_code)]
#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterDocRequest {
    #[schema(example = "user@rejki.id")]
    pub email: String,
    #[schema(example = "rahasia123")]
    pub password: String,
    pub phone: Option<String>,
    #[schema(example = true)]
    pub tos_accepted: bool,
    pub tos_version: Option<String>,
}

/// Payload login (`POST /api/v1/auth/login`).
#[allow(dead_code)]
#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginDocRequest {
    #[schema(example = "user@rejki.id")]
    pub email: String,
    #[schema(example = "rahasia123")]
    pub password: String,
}

/// Payload login admin (`POST /api/v1/auth/admin/login`).
#[allow(dead_code)]
#[derive(Debug, Deserialize, ToSchema)]
pub struct AdminLoginDocRequest {
    #[schema(example = "admin@rejki.id")]
    pub email: String,
    #[schema(example = "AdminPass123!")]
    pub password: String,
}

/// Respons token login / refresh.
#[derive(Debug, Serialize, ToSchema)]
pub struct LoginDocResponse {
    pub access_token: String,
    pub refresh_token: String,
    #[schema(example = "Bearer")]
    pub token_type: String,
    #[schema(example = 900)]
    pub expires_in: u64,
}

/// Payload verifikasi OTP (`POST /api/v1/auth/verify-otp`).
/// purpose divalidasi: "register" | "reset_password" | "change_password".
#[allow(dead_code)]
#[derive(Debug, Deserialize, ToSchema)]
pub struct VerifyOtpDocRequest {
    #[schema(example = "user@rejki.id")]
    pub email: String,
    #[schema(example = "123456")]
    pub otp: String,
    #[schema(example = "register")]
    pub purpose: String,
}

/// Payload OTP ulang (`POST /api/v1/auth/resend-otp`).
/// purpose divalidasi: "register" | "reset_password" | "change_password".
#[allow(dead_code)]
#[derive(Debug, Deserialize, ToSchema)]
pub struct ResendOtpDocRequest {
    #[schema(example = "user@rejki.id")]
    pub email: String,
    #[schema(example = "register")]
    pub purpose: String,
}

/// Payload refresh token (`POST /api/v1/auth/refresh`).
#[allow(dead_code)]
#[derive(Debug, Deserialize, ToSchema)]
pub struct RefreshDocRequest {
    pub refresh_token: String,
}

/// Payload lupa password (`POST /api/v1/auth/forgot-password`).
#[allow(dead_code)]
#[derive(Debug, Deserialize, ToSchema)]
pub struct ForgotPasswordDocRequest {
    #[schema(example = "user@rejki.id")]
    pub email: String,
}

/// Payload reset password (`POST /api/v1/auth/reset-password`).
#[allow(dead_code)]
#[derive(Debug, Deserialize, ToSchema)]
pub struct ResetPasswordDocRequest {
    #[schema(example = "user@rejki.id")]
    pub email: String,
    #[schema(example = "123456")]
    pub otp: String,
    #[schema(example = "R4has1aBaru!")]
    pub new_password: String,
}

/// Payload ubah password (`POST /api/v1/auth/change-password`).
#[allow(dead_code)]
#[derive(Debug, Deserialize, ToSchema)]
pub struct ChangePasswordDocRequest {
    #[schema(example = "123456")]
    pub otp: String,
    #[schema(example = "R4has1aBaru!")]
    pub new_password: String,
}

/// Payload suspend akun (`POST /api/v1/auth/admin/users/{id}/suspend`).
/// evidence_object_key WAJIB diisi — unggah bukti via endpoint evidence terlebih dahulu.
#[allow(dead_code)]
#[derive(Debug, Deserialize, ToSchema)]
pub struct SuspendDocRequest {
    #[schema(example = false)]
    pub permanent: bool,
    #[schema(example = "Melanggar Pedoman Komunitas Pasal 3")]
    pub reason: String,
    pub expires_at: Option<String>,
    /// WAJIB — object key dari endpoint evidence presigned upload.
    #[schema(example = "suspension-evidence/2026/06/abc123.jpg")]
    pub evidence_object_key: String,
}

/// Payload bulk suspend pengguna (`POST /api/v1/auth/admin/users/suspend`).
/// evidence_object_key WAJIB diisi.
#[allow(dead_code)]
#[derive(Debug, Deserialize, ToSchema)]
pub struct BulkSuspendDocRequest {
    pub user_ids: Vec<String>,
    #[schema(example = false)]
    pub permanent: bool,
    #[schema(example = "Melanggar Pedoman Komunitas Pasal 3")]
    pub reason: String,
    pub expires_at: Option<String>,
    /// WAJIB — object key dari endpoint evidence presigned upload.
    #[schema(example = "suspension-evidence/2026/06/abc123.jpg")]
    pub evidence_object_key: String,
}

/// Hasil suspend per-pengguna (partial-success).
#[allow(dead_code)]
#[derive(Debug, Serialize, ToSchema)]
pub struct BulkSuspendItemDocResponse {
    pub user_id: String,
    #[schema(example = true)]
    pub success: bool,
    pub error: Option<String>,
}

/// Respons bulk suspend — array hasil per-pengguna.
#[allow(dead_code)]
#[derive(Debug, Serialize, ToSchema)]
pub struct BulkSuspendDocResponse {
    pub results: Vec<BulkSuspendItemDocResponse>,
}

/// Payload bukti suspend — minta presigned URL (`POST /.../suspend/evidence`).
#[allow(dead_code)]
#[derive(Debug, Deserialize, ToSchema)]
pub struct SuspendEvidenceDocRequest {
    #[schema(example = "image/jpeg")]
    pub mime: String,
    pub size_bytes: u64,
}

// ── User / KYC ───────────────────────────────────────────────────────────────

/// Respons profil (`GET /api/v1/users/me`).
#[derive(Debug, Serialize, ToSchema)]
pub struct UserProfileDocResponse {
    pub id: uuid::Uuid,
    pub username: String,
    pub full_name: Option<String>,
    pub avatar: Option<String>,
    pub bio: Option<String>,
    pub phone: Option<String>,
    /// Peran pengguna (RBAC) — "user" atau "admin".
    pub role: Option<String>,
    pub nik_masked: Option<String>,
    pub kyc_status: Option<String>,
}

/// Payload update profil (`PATCH /api/v1/users/me`).
#[allow(dead_code)]
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateProfileDocRequest {
    pub full_name: Option<String>,
    pub avatar: Option<String>,
    pub bio: Option<String>,
    pub phone: Option<String>,
}

/// Payload kirim data KYC (`PUT /api/v1/users/me/kyc`).
#[allow(dead_code)]
#[derive(Debug, Deserialize, ToSchema)]
pub struct KycSubmitDocRequest {
    #[schema(example = "Budi Santoso")]
    pub full_name: String,
    #[schema(example = "3275012345678901")]
    pub nik: String,
    #[schema(example = "s1")]
    pub education_level: String,
    #[schema(example = "male")]
    pub gender: String,
    pub birth_date: String,
    #[schema(example = "Jl. Merdeka No. 10")]
    pub address_line: String,
    pub country_code: Option<String>,
    pub province_id: String,
    pub regency_id: String,
    pub district_id: String,
    pub village_id: String,
}

/// Respons submission KYC.
#[derive(Debug, Serialize, ToSchema)]
pub struct KycSubmissionDocResponse {
    pub id: uuid::Uuid,
    pub status: String,
    pub review_note: Option<String>,
    pub reviewed_at: Option<String>,
    pub created_at: String,
}

/// Item daftar pengajuan KYC untuk admin (`GET /api/v1/users/admin/kyc`).
/// NIK hanya ter-mask; id wilayah disertakan agar UI resolve nama via region-service.
#[derive(Debug, Serialize, ToSchema)]
pub struct AdminKycListItemDocResponse {
    /// ID pengajuan (submission).
    pub id: uuid::Uuid,
    pub full_name: Option<String>,
    pub education_level: Option<String>,
    pub gender: Option<String>,
    pub birth_date: Option<String>,
    pub address_line: Option<String>,
    pub country_code: String,
    pub province_id: Option<String>,
    pub regency_id: Option<String>,
    pub district_id: Option<String>,
    pub village_id: Option<String>,
    #[schema(example = "xxx...8901")]
    pub nik_masked: Option<String>,
    pub status: String,
    pub created_at: String,
}

/// Detail pengajuan KYC untuk pop-up admin (`GET /api/v1/users/admin/kyc/{id}`).
#[derive(Debug, Serialize, ToSchema)]
pub struct AdminKycDetailDocResponse {
    pub id: uuid::Uuid,
    pub profile_id: uuid::Uuid,
    pub full_name: Option<String>,
    pub education_level: Option<String>,
    pub gender: Option<String>,
    pub birth_date: Option<String>,
    pub address_line: Option<String>,
    pub country_code: String,
    pub province_id: Option<String>,
    pub regency_id: Option<String>,
    pub district_id: Option<String>,
    pub village_id: Option<String>,
    #[schema(example = "xxx...8901")]
    pub nik_masked: Option<String>,
    pub status: String,
    /// Apakah dokumen KTP tersedia (untuk click-to-view).
    pub has_ktp: bool,
    /// Apakah dokumen Selfie tersedia.
    pub has_selfie: bool,
    pub created_at: String,
}

/// Payload permintaan upload dokumen/avatar (`POST /api/v1/users/me/documents` / `/me/avatar`).
#[allow(dead_code)]
#[derive(Debug, Deserialize, ToSchema)]
pub struct UploadRequestDocRequest {
    pub kind: Option<String>,
    #[schema(example = "image/jpeg")]
    pub mime: String,
    pub size_bytes: u64,
}

/// Payload commit dokumen (`POST /api/v1/users/me/documents/commit`).
#[allow(dead_code)]
#[derive(Debug, Deserialize, ToSchema)]
pub struct CommitDocumentDocRequest {
    pub kind: String,
    pub object_key: String,
    #[schema(example = "image/jpeg")]
    pub mime: String,
    pub magic_head_b64: String,
}

/// Respons presigned URL (upload permission).
#[derive(Debug, Serialize, ToSchema)]
pub struct UploadPermissionDocResponse {
    pub presigned_url: String,
    pub object_key: String,
}

/// Payload review KYC oleh admin (`POST /api/v1/users/admin/kyc/{id}/review`).
#[allow(dead_code)]
#[derive(Debug, Deserialize, ToSchema)]
pub struct ReviewKycDocRequest {
    #[schema(example = true)]
    pub approved: bool,
    pub review_note: Option<String>,
}

// ── Notifikasi ────────────────────────────────────────────────────────────────

/// Payload notifikasi (contoh — baca daftar in-app).
#[derive(Debug, Serialize, ToSchema)]
pub struct NotifItemDocResponse {
    pub id: uuid::Uuid,
    pub title: String,
    pub body: String,
    pub created_at: String,
}

// ── Region ───────────────────────────────────────────────────────────────────

/// Item wilayah (provinsi/kota/kecamatan/kelurahan).
#[derive(Debug, Serialize, ToSchema)]
pub struct RegionDocItem {
    pub id: String,
    pub name: String,
}

// ── Path annotations ─────────────────────────────────────────────────────────

/// Health-check global.
#[utoipa::path(get, path = "/health", tag = "system",
    responses((status = 200, description = "Service sehat")))
]
#[allow(dead_code)]
fn health_doc() {}

/// POST /api/v1/auth/register — daftar akun baru.
#[utoipa::path(post, path = "/api/v1/auth/register", tag = "auth",
    request_body = RegisterDocRequest,
    responses(
        (status = 201, description = "Akun dibuat; OTP email dikirim"),
        (status = 422, description = "Validasi gagal (password lemah, email invalid, T&C belum disetujui)")
    )
)]
#[allow(dead_code)]
fn register_doc() {}

/// POST /api/v1/auth/login — login email+password.
/// Rate-limited (5 percobaan/15 menit per email). Anti-enumeration: semua kegagalan → 401.
#[utoipa::path(post, path = "/api/v1/auth/login", tag = "auth",
    request_body = LoginDocRequest,
    responses(
        (status = 200, description = "Login berhasil", body = LoginDocResponse),
        (status = 401, description = "Kredensial tidak valid / akun belum aktif / akun ditangguhkan (anti-enumeration)"),
        (status = 429, description = "Terlalu banyak percobaan — coba lagi nanti")
    )
)]
#[allow(dead_code)]
fn login_doc() {}

/// POST /api/v1/auth/admin/login — login admin email+password (tanpa OTP).
/// Rate-limited (5 percobaan/15 menit per email). Anti-enumeration + constant-time (timing-safe).
/// SuspendedTemp admin auto-recovery jika masa suspensi expired.
#[utoipa::path(post, path = "/api/v1/auth/admin/login", tag = "admin",
    request_body = AdminLoginDocRequest,
    responses(
        (status = 200, description = "Login admin berhasil; token memuat klaim role=admin", body = LoginDocResponse),
        (status = 401, description = "Kredensial salah / bukan admin / tidak aktif (anti-enumeration, timing-safe)"),
        (status = 429, description = "Terlalu banyak percobaan — coba lagi nanti")
    )
)]
#[allow(dead_code)]
fn admin_login_doc() {}

/// POST /api/v1/auth/verify-otp — verifikasi OTP.
#[utoipa::path(post, path = "/api/v1/auth/verify-otp", tag = "auth",
    request_body = VerifyOtpDocRequest,
    responses(
        (status = 200, description = "OTP valid"),
        (status = 422, description = "OTP tidak valid / expired")
    )
)]
#[allow(dead_code)]
fn verify_otp_doc() {}

/// POST /api/v1/auth/resend-otp — kirim ulang OTP. Rate-limited (3/15 menit per email).
#[utoipa::path(post, path = "/api/v1/auth/resend-otp", tag = "auth",
    request_body = ResendOtpDocRequest,
    responses(
        (status = 200, description = "OTP dikirim ulang (mengikuti rate limit)"),
        (status = 429, description = "Terlalu banyak permintaan OTP — coba lagi nanti")
    )
)]
#[allow(dead_code)]
fn resend_otp_doc() {}

/// POST /api/v1/auth/refresh — tukar refresh token dengan access token baru.
#[utoipa::path(post, path = "/api/v1/auth/refresh", tag = "auth",
    request_body = RefreshDocRequest,
    responses(
        (status = 200, description = "Access token baru", body = LoginDocResponse),
        (status = 401, description = "Refresh token invalid/expired")
    )
)]
#[allow(dead_code)]
fn refresh_doc() {}

/// POST /api/v1/auth/logout — cabut refresh token.
#[utoipa::path(post, path = "/api/v1/auth/logout", tag = "auth",
    responses((status = 204, description = "Refresh token dicabut"))
)]
#[allow(dead_code)]
fn logout_doc() {}

/// POST /api/v1/auth/forgot-password — minta reset password (anti-enumeration).
#[utoipa::path(post, path = "/api/v1/auth/forgot-password", tag = "auth",
    request_body = ForgotPasswordDocRequest,
    responses((status = 200, description = "Selalu 200 (anti-enumeration)"))
)]
#[allow(dead_code)]
fn forgot_password_doc() {}

/// POST /api/v1/auth/reset-password — set password baru dengan OTP.
#[utoipa::path(post, path = "/api/v1/auth/reset-password", tag = "auth",
    request_body = ResetPasswordDocRequest,
    responses((status = 200, description = "Password di-reset; sesi lama dicabut"))
)]
#[allow(dead_code)]
fn reset_password_doc() {}

/// POST /api/v1/auth/change-password — ubah password (authed).
#[utoipa::path(post, path = "/api/v1/auth/change-password", tag = "auth",
    request_body = ChangePasswordDocRequest,
    responses((status = 200, description = "Password diubah; sesi lain dicabut"))
)]
#[allow(dead_code)]
fn change_password_doc() {}

/// POST /api/v1/auth/admin/users/{id}/suspend — suspend satu akun (admin).
/// evidence_object_key WAJIB. Hanya admin Active yang bisa mengakses.
#[utoipa::path(post, path = "/api/v1/auth/admin/users/{id}/suspend", tag = "admin",
    request_body = SuspendDocRequest,
    responses(
        (status = 200, description = "Akun di-suspend; token dicabut + notifikasi email/in-app"),
        (status = 403, description = "Bukan admin / akun admin tidak aktif (ACCOUNT_NOT_ADMIN / ACCOUNT_NOT_ACTIVE)"),
        (status = 422, description = "Validasi gagal (reason kosong / expires_at kosong saat sementara / evidence_object_key wajib)")
    )
)]
#[allow(dead_code)]
fn suspend_doc() {}

/// POST /api/v1/auth/admin/users/suspend — suspend massal pengguna (admin, partial-success).
/// evidence_object_key WAJIB. Partial-success: hasil per-item di response.
#[utoipa::path(post, path = "/api/v1/auth/admin/users/suspend", tag = "admin",
    request_body = BulkSuspendDocRequest,
    responses(
        (status = 200, description = "Hasil per-pengguna (partial-success)", body = BulkSuspendDocResponse),
        (status = 403, description = "Bukan admin / akun admin tidak aktif"),
        (status = 422, description = "Validasi gagal (reason kosong / expires_at kosong saat sementara / evidence_object_key wajib / >100 user)")
    )
)]
#[allow(dead_code)]
fn suspend_bulk_doc() {}

/// POST /api/v1/auth/admin/users/{id}/suspend/evidence — bukti suspend (Q1).
#[utoipa::path(post, path = "/api/v1/auth/admin/users/{id}/suspend/evidence", tag = "admin",
    request_body = SuspendEvidenceDocRequest,
    responses(
        (status = 200, description = "Presigned URL untuk upload bukti", body = UploadPermissionDocResponse),
        (status = 422, description = "MIME/ukuran tidak valid")
    )
)]
#[allow(dead_code)]
fn suspend_evidence_doc() {}

// ── Users ────────────────────────────────────────────────────────────────────

/// GET /api/v1/users/me — profil sendiri (+ status KYC, NIK ter-mask).
#[utoipa::path(get, path = "/api/v1/users/me", tag = "users",
    responses(
        (status = 200, description = "Profil lengkap dengan status KYC", body = UserProfileDocResponse),
        (status = 404, description = "Profil tidak ditemukan")
    )
)]
#[allow(dead_code)]
fn get_me_doc() {}

/// PATCH /api/v1/users/me — perbarui field profil non-KYC.
#[utoipa::path(patch, path = "/api/v1/users/me", tag = "users",
    request_body = UpdateProfileDocRequest,
    responses((status = 200, description = "Profil diperbarui", body = UserProfileDocResponse))
)]
#[allow(dead_code)]
fn update_me_doc() {}

/// PUT /api/v1/users/me/kyc — kirim data diri KYC (submission).
#[utoipa::path(put, path = "/api/v1/users/me/kyc", tag = "kyc",
    request_body = KycSubmitDocRequest,
    responses(
        (status = 201, description = "Data KYC dikirim; menunggu verifikasi", body = KycSubmissionDocResponse),
        (status = 422, description = "Validasi gagal / NIK sudah di-set / rantai wilayah tidak konsisten")
    )
)]
#[allow(dead_code)]
fn submit_kyc_doc() {}

/// GET /api/v1/users/me/kyc/status — status KYC terkini.
#[utoipa::path(get, path = "/api/v1/users/me/kyc/status", tag = "kyc",
    responses((status = 200, description = "Status KYC terkini", body = KycSubmissionDocResponse))
)]
#[allow(dead_code)]
fn kyc_status_doc() {}

/// POST /api/v1/users/me/avatar — minta presigned URL upload avatar.
#[utoipa::path(post, path = "/api/v1/users/me/avatar", tag = "users",
    request_body = UploadRequestDocRequest,
    responses(
        (status = 200, description = "Presigned URL upload avatar", body = UploadPermissionDocResponse),
        (status = 422, description = "MIME/ukuran tidak valid")
    )
)]
#[allow(dead_code)]
fn avatar_doc() {}

/// POST /api/v1/users/me/documents — minta presigned URL upload dokumen KYC.
#[utoipa::path(post, path = "/api/v1/users/me/documents", tag = "kyc",
    request_body = UploadRequestDocRequest,
    responses(
        (status = 200, description = "Presigned URL upload dokumen", body = UploadPermissionDocResponse),
        (status = 422, description = "Jenis dokumen invalid / MIME/ukuran tidak valid")
    )
)]
#[allow(dead_code)]
fn request_doc_upload_doc() {}

/// POST /api/v1/users/me/documents/commit — commit dokumen (magic bytes verificaton).
#[utoipa::path(post, path = "/api/v1/users/me/documents/commit", tag = "kyc",
    request_body = CommitDocumentDocRequest,
    responses(
        (status = 200, description = "Dokumen di-commit; object key tersimpan"),
        (status = 422, description = "Magic bytes tidak cocok / object_key tidak valid")
    )
)]
#[allow(dead_code)]
fn commit_document_doc() {}

/// GET /api/v1/users/me/documents/{kind} — akses baca dokumen (presigned temporary URL).
#[utoipa::path(get, path = "/api/v1/users/me/documents/{kind}", tag = "kyc",
    responses(
        (status = 200, description = "Presigned read URL", body = UploadPermissionDocResponse),
        (status = 404, description = "Dokumen belum diupload")
    )
)]
#[allow(dead_code)]
fn doc_read_doc() {}

/// POST /api/v1/users/admin/kyc/{id}/review — admin review KYC (approve/reject).
/// Idempoten: pengajuan terminal (approved/rejected) mengembalikan 409.
#[utoipa::path(post, path = "/api/v1/users/admin/kyc/{id}/review", tag = "admin",
    request_body = ReviewKycDocRequest,
    responses(
        (status = 200, description = "KYC direview; notifikasi dikirim ke user"),
        (status = 404, description = "Submission tidak ditemukan"),
        (status = 409, description = "Pengajuan sudah ditinjau (terminal) — tidak dapat ditinjau ulang")
    )
)]
#[allow(dead_code)]
fn review_kyc_doc() {}

/// GET /api/v1/users/admin/kyc — daftar pengajuan KYC (admin; search/filter/sort/paginate).
#[utoipa::path(get, path = "/api/v1/users/admin/kyc", tag = "admin",
    params(
        ("q" = Option<String>, Query, description = "Pencarian: Nama / ID submission / ID profil"),
        ("status" = Option<String>, Query, description = "Filter status: pending|approved|rejected (default pending)"),
        ("sort_dir" = Option<String>, Query, description = "Arah urut created_at: asc|desc (default desc)"),
        ("limit" = Option<i64>, Query, description = "Batas baris (default 20)"),
        ("offset" = Option<i64>, Query, description = "Offset paginasi (default 0)")
    ),
    responses(
        (status = 200, description = "Daftar pengajuan KYC (NIK ter-mask) + meta paginasi", body = Vec<AdminKycListItemDocResponse>),
        (status = 403, description = "Bukan admin")
    )
)]
#[allow(dead_code)]
fn admin_list_kyc_doc() {}

/// GET /api/v1/users/admin/kyc/export.csv — ekspor daftar pengajuan KYC sesuai filter aktif.
#[utoipa::path(get, path = "/api/v1/users/admin/kyc/export.csv", tag = "admin",
    responses(
        (status = 200, description = "Berkas CSV daftar pengajuan KYC", content_type = "text/csv"),
        (status = 403, description = "Bukan admin")
    )
)]
#[allow(dead_code)]
fn admin_export_kyc_doc() {}

/// GET /api/v1/users/admin/kyc/{id} — detail satu pengajuan KYC (pop-up; NIK ter-mask).
#[utoipa::path(get, path = "/api/v1/users/admin/kyc/{id}", tag = "admin",
    responses(
        (status = 200, description = "Detail pengajuan KYC", body = AdminKycDetailDocResponse),
        (status = 403, description = "Bukan admin"),
        (status = 404, description = "Pengajuan tidak ditemukan")
    )
)]
#[allow(dead_code)]
fn admin_get_kyc_doc() {}

/// GET /api/v1/users/admin/kyc/{id}/documents/{kind} — presigned read URL dokumen (teraudit).
#[utoipa::path(get, path = "/api/v1/users/admin/kyc/{id}/documents/{kind}", tag = "admin",
    params(
        ("kind" = String, Path, description = "Jenis dokumen: ktp | selfie")
    ),
    responses(
        (status = 200, description = "URL akses sementara dokumen; akses dicatat audit (aktor admin)"),
        (status = 403, description = "Bukan admin"),
        (status = 404, description = "Dokumen tidak tersedia / sudah dimusnahkan"),
        (status = 422, description = "Jenis dokumen tidak dikenal")
    )
)]
#[allow(dead_code)]
fn admin_get_document_doc() {}

/// GET /api/v1/users/{id} — lihat profil pengguna sendiri (C3: protected, ownership check).
#[utoipa::path(get, path = "/api/v1/users/{id}", tag = "users",
    responses(
        (status = 200, description = "Profil pengguna (pemilik)", body = UserProfileDocResponse),
        (status = 401, description = "Token tidak ada atau tidak valid"),
        (status = 404, description = "Profil tidak ditemukan (bukan pemilik)")
    )
)]
#[allow(dead_code)]
fn get_by_id_doc() {}

// ── Region ───────────────────────────────────────────────────────────────────

/// GET /api/v1/regions/provinces — daftar provinsi (cascading level 1).
#[utoipa::path(get, path = "/api/v1/regions/provinces", tag = "regions",
    responses((status = 200, description = "Daftar provinsi", body = Vec<RegionDocItem>))
)]
#[allow(dead_code)]
fn provinces_doc() {}

/// GET /api/v1/regions/regencies — daftar kabupaten/kota per provinsi.
#[utoipa::path(get, path = "/api/v1/regions/regencies", tag = "regions",
    responses((status = 200, description = "Daftar kabupaten/kota", body = Vec<RegionDocItem>))
)]
#[allow(dead_code)]
fn regencies_doc() {}

/// GET /api/v1/regions/districts — daftar kecamatan per kota.
#[utoipa::path(get, path = "/api/v1/regions/districts", tag = "regions",
    responses((status = 200, description = "Daftar kecamatan", body = Vec<RegionDocItem>))
)]
#[allow(dead_code)]
fn districts_doc() {}

/// GET /api/v1/regions/villages — daftar kelurahan per kecamatan.
#[utoipa::path(get, path = "/api/v1/regions/villages", tag = "regions",
    responses((status = 200, description = "Daftar kelurahan", body = Vec<RegionDocItem>))
)]
#[allow(dead_code)]
fn villages_doc() {}

// ── Notifikasi ────────────────────────────────────────────────────────────────

/// GET /api/v1/notif — daftar notifikasi in-app user.
#[utoipa::path(get, path = "/api/v1/notif", tag = "notifications",
    responses((status = 200, description = "Daftar notifikasi in-app", body = Vec<NotifItemDocResponse>))
)]
#[allow(dead_code)]
fn notif_doc() {}

// ── Iklan: mirror DTO ─────────────────────────────────────────────────────────

/// Respons iklan pekerjaan (publik).
#[derive(Debug, Serialize, ToSchema)]
pub struct IklanPekerjaanDocResponse {
    pub id: uuid::Uuid,
    pub poster_id: uuid::Uuid,
    pub judul: String,
    pub perusahaan: String,
    pub deskripsi: String,
    pub lokasi: Option<String>,
    pub gaji_min: Option<i64>,
    pub gaji_max: Option<i64>,
    pub tipe: String,
    #[schema(example = json!(["https://s3.example.com/foto1.jpg"]))]
    pub foto_urls: Vec<String>,
    pub is_active: bool,
    pub moderation_status: String,
    pub created_at: String,
}

/// Respons iklan pekerjaan (admin — menyertakan deleted_at).
#[derive(Debug, Serialize, ToSchema)]
pub struct AdminIklanDocResponse {
    pub id: uuid::Uuid,
    pub poster_id: uuid::Uuid,
    pub judul: String,
    pub perusahaan: String,
    pub deskripsi: String,
    pub lokasi: Option<String>,
    pub foto_urls: Vec<String>,
    pub is_active: bool,
    pub moderation_status: String,
    pub deleted_at: Option<String>,
    pub created_at: String,
}

/// Payload buat iklan.
#[allow(dead_code)]
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateIklanDocRequest {
    #[schema(example = "Lowongan Backend Developer")]
    pub judul: String,
    pub deskripsi: String,
    pub lokasi: Option<String>,
    pub foto_urls: Option<Vec<String>>,
}

/// Payload suspend iklan (admin).
#[allow(dead_code)]
#[derive(Debug, Deserialize, ToSchema)]
pub struct SuspendIklanDocRequest {
    pub iklan_ids: Vec<uuid::Uuid>,
    #[schema(example = false)]
    pub is_permanent: bool,
    #[schema(example = "Iklan melanggar Pedoman Komunitas Pasal 3")]
    pub reason: String,
    pub evidence_object_key: String,
    pub expires_at: Option<String>,
}

/// Respons suspend iklan per-item.
#[derive(Debug, Serialize, ToSchema)]
pub struct SuspendIklanItemDocResponse {
    pub iklan_id: uuid::Uuid,
    pub success: bool,
    pub error: Option<String>,
}

/// Respons suspend iklan (array hasil).
#[derive(Debug, Serialize, ToSchema)]
pub struct SuspendIklanDocResponse {
    pub results: Vec<SuspendIklanItemDocResponse>,
}

// ── Iklan: path annotations (public) ───────────────────────────────────────────

/// GET /api/v1/pekerjaan — daftar iklan publik.
#[utoipa::path(get, path = "/api/v1/pekerjaan", tag = "iklan-pekerjaan",
    params(
        ("limit" = Option<i64>, Query, description = "Items per halaman (default 20)"),
        ("offset" = Option<i64>, Query, description = "Offset halaman (default 0)"),
    ),
    responses((status = 200, description = "Daftar iklan pekerjaan", body = Vec<IklanPekerjaanDocResponse>))
)]
#[allow(dead_code)]
fn list_pekerjaan_doc() {}

/// GET /api/v1/pekerjaan/{id} — detail iklan.
#[utoipa::path(get, path = "/api/v1/pekerjaan/{id}", tag = "iklan-pekerjaan",
    responses(
        (status = 200, description = "Iklan pekerjaan", body = IklanPekerjaanDocResponse),
        (status = 404, description = "Tidak ditemukan")
    )
)]
#[allow(dead_code)]
fn get_pekerjaan_doc() {}

/// POST /api/v1/pekerjaan — buat iklan baru (authed).
#[utoipa::path(post, path = "/api/v1/pekerjaan", tag = "iklan-pekerjaan",
    request_body = CreateIklanDocRequest,
    responses((status = 201, description = "Iklan dibuat"))
)]
#[allow(dead_code)]
fn create_pekerjaan_doc() {}

// ── Iklan: path annotations (admin) ───────────────────────────────────────────

/// GET /api/v1/admin/pekerjaan — listing admin (search, filter, sort, paginate).
#[utoipa::path(get, path = "/api/v1/admin/pekerjaan", tag = "admin-iklan",
    params(
        ("q" = Option<String>, Query, description = "Pencarian (judul / kode / pembuat)"),
        ("status" = Option<String>, Query, description = "Filter status moderasi"),
        ("sort_by" = Option<String>, Query, description = "Kolom sort"),
        ("sort_dir" = Option<String>, Query, description = "Arah sort (asc/desc)"),
        ("limit" = Option<i64>, Query, description = "Items per halaman"),
        ("offset" = Option<i64>, Query, description = "Offset halaman"),
    ),
    responses((status = 200, description = "Daftar iklan (admin view)", body = Vec<AdminIklanDocResponse>))
)]
#[allow(dead_code)]
fn admin_list_pekerjaan_doc() {}

/// GET /api/v1/admin/pekerjaan/export.csv
#[utoipa::path(get, path = "/api/v1/admin/pekerjaan/export.csv", tag = "admin-iklan",
    params(
        ("q" = Option<String>, Query),
        ("status" = Option<String>, Query),
    ),
    responses((status = 200, description = "File CSV", content_type = "text/csv"))
)]
#[allow(dead_code)]
fn export_csv_pekerjaan_doc() {}

/// POST /api/v1/admin/pekerjaan/suspend/evidence — presigned URL upload bukti.
#[utoipa::path(post, path = "/api/v1/admin/pekerjaan/suspend/evidence", tag = "admin-iklan",
    request_body = SuspendEvidenceDocRequest,
    responses(
        (status = 200, description = "Presigned URL", body = UploadPermissionDocResponse),
        (status = 422, description = "MIME/ukuran tidak valid")
    )
)]
#[allow(dead_code)]
fn suspend_evidence_pekerjaan_doc() {}

/// POST /api/v1/admin/pekerjaan/suspend — suspend iklan (single/bulk).
#[utoipa::path(post, path = "/api/v1/admin/pekerjaan/suspend", tag = "admin-iklan",
    request_body = SuspendIklanDocRequest,
    responses((status = 200, description = "Hasil suspend per item", body = SuspendIklanDocResponse))
)]
#[allow(dead_code)]
fn suspend_pekerjaan_doc() {}

// ── Pelatihan: mirror DTO ──────────────────────────────────────────────────────
// Ref: openspec/changes/add-pelatihan-enrollment-badge

/// Respons pelatihan (publik).
#[derive(Debug, Serialize, ToSchema)]
pub struct IklanPelatihanDocResponse {
    pub id: uuid::Uuid,
    pub poster_id: uuid::Uuid,
    pub judul: String,
    pub penyelenggara: String,
    pub deskripsi: String,
    pub lokasi: Option<String>,
    pub harga: Option<i64>,
    pub tanggal_mulai: Option<String>,
    pub tanggal_selesai: Option<String>,
    pub foto_urls: Vec<String>,
    pub is_active: bool,
    pub status: String,
    pub created_by_role: String,
    pub jumlah_peserta: Option<i32>,
    pub created_at: String,
}

/// Respons pelatihan admin (lengkap dgn audit).
#[derive(Debug, Serialize, ToSchema)]
pub struct AdminPelatihanDocResponse {
    pub id: uuid::Uuid,
    pub poster_id: uuid::Uuid,
    pub judul: String,
    pub penyelenggara: String,
    pub deskripsi: String,
    pub lokasi: Option<String>,
    pub harga: Option<i64>,
    pub tanggal_mulai: Option<String>,
    pub tanggal_selesai: Option<String>,
    pub foto_urls: Vec<String>,
    pub is_active: bool,
    pub status: String,
    pub created_by_role: String,
    pub jumlah_peserta: Option<i32>,
    pub deleted_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Payload create pelatihan.
#[allow(dead_code)]
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreatePelatihanDocRequest {
    pub judul: String,
    pub penyelenggara: String,
    pub deskripsi: String,
    pub lokasi: Option<String>,
    pub harga: Option<i64>,
    pub tanggal_mulai: Option<String>,
    pub tanggal_selesai: Option<String>,
    pub foto_urls: Option<Vec<String>>,
    pub jumlah_peserta: Option<i32>,
}

/// Payload update pelatihan.
#[allow(dead_code)]
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdatePelatihanDocRequest {
    pub judul: String,
    pub penyelenggara: String,
    pub deskripsi: String,
    pub lokasi: Option<String>,
    pub harga: Option<i64>,
    pub tanggal_mulai: Option<String>,
    pub tanggal_selesai: Option<String>,
    pub jumlah_peserta: Option<i32>,
}

/// Payload review pelatihan (approve/reject).
#[allow(dead_code)]
#[derive(Debug, Deserialize, ToSchema)]
pub struct ReviewPelatihanDocRequest {
    pub approved: bool,
    pub review_note: Option<String>,
}

/// Respons enrollment.
#[derive(Debug, Serialize, ToSchema)]
pub struct EnrollmentDocResponse {
    pub id: uuid::Uuid,
    pub pelatihan_id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub bukti_transfer_object_key: Option<String>,
    pub status: String,
    pub reviewed_by: Option<uuid::Uuid>,
    pub review_note: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Payload review enrollment.
#[allow(dead_code)]
#[derive(Debug, Deserialize, ToSchema)]
pub struct ReviewEnrollmentDocRequest {
    pub approved: bool,
    pub review_note: Option<String>,
}

/// Respons badge.
#[derive(Debug, Serialize, ToSchema)]
pub struct BadgeDocResponse {
    pub id: uuid::Uuid,
    pub pelatihan_id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub sertifikat_object_key: Option<String>,
    pub approved_at: Option<String>,
    pub status: String,
    pub reviewed_by: Option<uuid::Uuid>,
    pub review_note: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Payload review badge.
#[allow(dead_code)]
#[derive(Debug, Deserialize, ToSchema)]
pub struct ReviewBadgeDocRequest {
    pub approved: bool,
    pub review_note: Option<String>,
}

/// Payload request upload evidence.
#[allow(dead_code)]
#[derive(Debug, Deserialize, ToSchema)]
pub struct PelatihanEvidenceDocRequest {
    pub mime: String,
    pub size_bytes: u64,
}

/// GET /api/v1/pelatihan — daftar pelatihan publik.
#[utoipa::path(get, path = "/api/v1/pelatihan/", tag = "pelatihan",
    responses((status = 200, description = "Daftar pelatihan", body = Vec<IklanPelatihanDocResponse>))
)]
#[allow(dead_code)]
fn list_pelatihan_doc() {}

/// GET /api/v1/pelatihan/{id} — detail pelatihan.
#[utoipa::path(get, path = "/api/v1/pelatihan/{id}", tag = "pelatihan",
    params(("id" = uuid::Uuid, Path, description = "ID pelatihan")),
    responses((status = 200, description = "Detail pelatihan", body = IklanPelatihanDocResponse))
)]
#[allow(dead_code)]
fn get_pelatihan_doc() {}

/// POST /api/v1/pelatihan/ — user create pelatihan.
#[utoipa::path(post, path = "/api/v1/pelatihan/", tag = "pelatihan",
    request_body = CreatePelatihanDocRequest,
    responses((status = 201, description = "Pelatihan dibuat oleh user", body = IklanPelatihanDocResponse))
)]
#[allow(dead_code)]
fn create_pelatihan_doc() {}

/// POST /api/v1/pelatihan/{id}/enroll — daftar pelatihan.
#[utoipa::path(post, path = "/api/v1/pelatihan/{id}/enroll", tag = "pelatihan",
    params(("id" = uuid::Uuid, Path, description = "ID pelatihan")),
    responses((status = 201, description = "Enrollment berhasil", body = EnrollmentDocResponse))
)]
#[allow(dead_code)]
fn enroll_pelatihan_doc() {}

/// POST /api/v1/pelatihan/{id}/badge — ajukan badge.
#[utoipa::path(post, path = "/api/v1/pelatihan/{id}/badge", tag = "pelatihan",
    params(("id" = uuid::Uuid, Path, description = "ID pelatihan")),
    responses((status = 201, description = "Badge berhasil diajukan", body = BadgeDocResponse))
)]
#[allow(dead_code)]
fn badge_pelatihan_doc() {}

// ── Admin: Pelatihan ──────────────────────────────────────────────────────────

/// GET /api/v1/pelatihan/admin/pelatihan — daftar pelatihan admin (7-stage).
#[utoipa::path(get, path = "/api/v1/pelatihan/admin/pelatihan", tag = "admin-pelatihan",
    responses((status = 200, description = "Daftar pelatihan admin", body = Vec<AdminPelatihanDocResponse>))
)]
#[allow(dead_code)]
fn admin_list_pelatihan_doc() {}

/// POST /api/v1/pelatihan/admin/pelatihan — admin create pelatihan (auto-approve).
#[utoipa::path(post, path = "/api/v1/pelatihan/admin/pelatihan", tag = "admin-pelatihan",
    request_body = CreatePelatihanDocRequest,
    responses((status = 201, description = "Pelatihan dibuat oleh admin (auto-approve)", body = AdminPelatihanDocResponse))
)]
#[allow(dead_code)]
fn admin_create_pelatihan_doc() {}

/// PATCH /api/v1/pelatihan/admin/pelatihan/{id} — admin edit pelatihan miliknya.
#[utoipa::path(patch, path = "/api/v1/pelatihan/admin/pelatihan/{id}", tag = "admin-pelatihan",
    params(("id" = uuid::Uuid, Path, description = "ID pelatihan")),
    request_body = UpdatePelatihanDocRequest,
    responses((status = 200, description = "Pelatihan diupdate", body = AdminPelatihanDocResponse))
)]
#[allow(dead_code)]
fn admin_update_pelatihan_doc() {}

/// DELETE /api/v1/pelatihan/admin/pelatihan/{id} — admin cancel pelatihan miliknya.
#[utoipa::path(delete, path = "/api/v1/pelatihan/admin/pelatihan/{id}", tag = "admin-pelatihan",
    params(("id" = uuid::Uuid, Path, description = "ID pelatihan")),
    responses((status = 204, description = "Pelatihan dicancel"))
)]
#[allow(dead_code)]
fn admin_cancel_pelatihan_doc() {}

/// POST /api/v1/pelatihan/admin/pelatihan/{id}/review — admin review pelatihan.
#[utoipa::path(post, path = "/api/v1/pelatihan/admin/pelatihan/{id}/review", tag = "admin-pelatihan",
    params(("id" = uuid::Uuid, Path, description = "ID pelatihan")),
    request_body = ReviewPelatihanDocRequest,
    responses((status = 200, description = "Hasil review pelatihan", body = AdminPelatihanDocResponse))
)]
#[allow(dead_code)]
fn admin_review_pelatihan_doc() {}

/// GET /api/v1/pelatihan/admin/pelatihan/export.csv — export CSV pelatihan.
#[utoipa::path(get, path = "/api/v1/pelatihan/admin/pelatihan/export.csv", tag = "admin-pelatihan",
    responses((status = 200, description = "File CSV pelatihan"))
)]
#[allow(dead_code)]
fn admin_export_pelatihan_doc() {}

/// GET /api/v1/pelatihan/admin/enrollments — daftar enrollment admin.
#[utoipa::path(get, path = "/api/v1/pelatihan/admin/enrollments", tag = "admin-pelatihan",
    responses((status = 200, description = "Daftar enrollment", body = Vec<EnrollmentDocResponse>))
)]
#[allow(dead_code)]
fn admin_list_enrollment_doc() {}

/// GET /api/v1/pelatihan/admin/enrollments/{id} — detail enrollment.
#[utoipa::path(get, path = "/api/v1/pelatihan/admin/enrollments/{id}", tag = "admin-pelatihan",
    params(("id" = uuid::Uuid, Path, description = "ID enrollment")),
    responses((status = 200, description = "Detail enrollment", body = EnrollmentDocResponse))
)]
#[allow(dead_code)]
fn admin_detail_enrollment_doc() {}

/// POST /api/v1/pelatihan/admin/enrollments/{id}/review — admin review enrollment.
#[utoipa::path(post, path = "/api/v1/pelatihan/admin/enrollments/{id}/review", tag = "admin-pelatihan",
    params(("id" = uuid::Uuid, Path, description = "ID enrollment")),
    request_body = ReviewEnrollmentDocRequest,
    responses((status = 200, description = "Hasil review enrollment", body = EnrollmentDocResponse))
)]
#[allow(dead_code)]
fn admin_review_enrollment_doc() {}

/// GET /api/v1/pelatihan/admin/enrollments/export.csv — export CSV enrollment.
#[utoipa::path(get, path = "/api/v1/pelatihan/admin/enrollments/export.csv", tag = "admin-pelatihan",
    responses((status = 200, description = "File CSV enrollment"))
)]
#[allow(dead_code)]
fn admin_export_enrollment_doc() {}

/// GET /api/v1/pelatihan/admin/badges — daftar badge admin.
#[utoipa::path(get, path = "/api/v1/pelatihan/admin/badges", tag = "admin-pelatihan",
    responses((status = 200, description = "Daftar badge", body = Vec<BadgeDocResponse>))
)]
#[allow(dead_code)]
fn admin_list_badge_doc() {}

/// GET /api/v1/pelatihan/admin/badges/{id} — detail badge.
#[utoipa::path(get, path = "/api/v1/pelatihan/admin/badges/{id}", tag = "admin-pelatihan",
    params(("id" = uuid::Uuid, Path, description = "ID badge")),
    responses((status = 200, description = "Detail badge", body = BadgeDocResponse))
)]
#[allow(dead_code)]
fn admin_detail_badge_doc() {}

/// POST /api/v1/pelatihan/admin/badges/{id}/review — admin review badge.
#[utoipa::path(post, path = "/api/v1/pelatihan/admin/badges/{id}/review", tag = "admin-pelatihan",
    params(("id" = uuid::Uuid, Path, description = "ID badge")),
    request_body = ReviewBadgeDocRequest,
    responses((status = 200, description = "Hasil review badge", body = BadgeDocResponse))
)]
#[allow(dead_code)]
fn admin_review_badge_doc() {}

/// GET /api/v1/pelatihan/admin/badges/export.csv — export CSV badge.
#[utoipa::path(get, path = "/api/v1/pelatihan/admin/badges/export.csv", tag = "admin-pelatihan",
    responses((status = 200, description = "File CSV badge"))
)]
#[allow(dead_code)]
fn admin_export_badge_doc() {}

// ── Corporate Comms: mirror DTO ────────────────────────────────────────────────
// Ref: openspec/changes/add-corporate-comms

/// Respons artikel (admin view).
#[derive(Debug, Serialize, ToSchema)]
pub struct AdminArticleDocResponse {
    pub id: uuid::Uuid,
    pub author_id: uuid::Uuid,
    /// Kategori artikel — saat ini hanya "informasi".
    pub category: String,
    pub title: String,
    pub body: String,
    pub photo_object_key: Option<String>,
    pub deleted_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Payload buat artikel.
#[allow(dead_code)]
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateArticleDocRequest {
    #[schema(example = "Pengumuman Penting")]
    pub title: String,
    #[schema(example = "Kepada seluruh pengguna Rejki...")]
    pub body: String,
    /// Kategori artikel — default "informasi".
    #[schema(example = "informasi")]
    pub category: Option<String>,
}

/// Payload update artikel.
#[allow(dead_code)]
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateArticleDocRequest {
    #[schema(example = "Pengumuman Penting (Revisi)")]
    pub title: String,
    #[schema(example = "Kepada seluruh pengguna Rejki, kami informasikan bahwa...")]
    pub body: String,
    #[schema(example = "informasi")]
    pub category: String,
    pub photo_object_key: Option<String>,
}

/// Payload permintaan upload foto artikel.
#[allow(dead_code)]
#[derive(Debug, Deserialize, ToSchema)]
pub struct ArticlePhotoDocRequest {
    #[schema(example = "image/jpeg")]
    pub mime: String,
    pub size_bytes: u64,
}

// ── Corporate Comms: path annotations ──────────────────────────────────────────

/// GET /api/v1/admin/articles — daftar artikel (search judul, sort kategori, paginasi).
#[utoipa::path(get, path = "/api/v1/admin/articles", tag = "admin-corporate-comms",
    params(
        ("q" = Option<String>, Query, description = "Cari berdasarkan judul (ILIKE)"),
        ("category" = Option<String>, Query, description = "Filter kategori (saat ini hanya 'informasi')"),
        ("sort_dir" = Option<String>, Query, description = "Arah sort kategori (asc/desc)"),
        ("limit" = Option<i64>, Query, description = "Items per halaman (default 20)"),
        ("offset" = Option<i64>, Query, description = "Offset halaman (default 0)"),
    ),
    responses((status = 200, description = "Daftar artikel (admin view)", body = Vec<AdminArticleDocResponse>))
)]
#[allow(dead_code)]
fn admin_list_articles_doc() {}

/// POST /api/v1/admin/articles — buat artikel baru (author dari sesi admin).
#[utoipa::path(post, path = "/api/v1/admin/articles", tag = "admin-corporate-comms",
    request_body = CreateArticleDocRequest,
    responses(
        (status = 201, description = "Artikel dibuat; broadcast notifikasi ke seluruh pengguna"),
        (status = 422, description = "Validasi gagal (judul min 3, body min 1)")
    )
)]
#[allow(dead_code)]
fn admin_create_article_doc() {}

/// POST /api/v1/admin/articles/photo-upload — minta presigned URL upload foto artikel.
#[utoipa::path(post, path = "/api/v1/admin/articles/photo-upload", tag = "admin-corporate-comms",
    request_body = ArticlePhotoDocRequest,
    responses(
        (status = 200, description = "Presigned URL upload foto artikel", body = UploadPermissionDocResponse),
        (status = 422, description = "MIME/ukuran tidak valid (≤5MB, JPEG/PNG)")
    )
)]
#[allow(dead_code)]
fn article_photo_upload_doc() {}

/// GET /api/v1/admin/articles/{id} — detail artikel.
#[utoipa::path(get, path = "/api/v1/admin/articles/{id}", tag = "admin-corporate-comms",
    params(("id" = uuid::Uuid, Path, description = "ID artikel")),
    responses(
        (status = 200, description = "Detail artikel", body = AdminArticleDocResponse),
        (status = 404, description = "Artikel tidak ditemukan")
    )
)]
#[allow(dead_code)]
fn admin_get_article_doc() {}

/// PATCH /api/v1/admin/articles/{id} — sunting artikel.
#[utoipa::path(patch, path = "/api/v1/admin/articles/{id}", tag = "admin-corporate-comms",
    params(("id" = uuid::Uuid, Path, description = "ID artikel")),
    request_body = UpdateArticleDocRequest,
    responses(
        (status = 200, description = "Artikel disunting; broadcast notifikasi ke seluruh pengguna", body = AdminArticleDocResponse),
        (status = 404, description = "Artikel tidak ditemukan")
    )
)]
#[allow(dead_code)]
fn admin_update_article_doc() {}

/// DELETE /api/v1/admin/articles/{id} — hapus artikel (soft-delete).
#[utoipa::path(delete, path = "/api/v1/admin/articles/{id}", tag = "admin-corporate-comms",
    params(("id" = uuid::Uuid, Path, description = "ID artikel")),
    responses(
        (status = 204, description = "Artikel dihapus (soft-delete)"),
        (status = 404, description = "Artikel tidak ditemukan")
    )
)]
#[allow(dead_code)]
fn admin_delete_article_doc() {}

// ── Content Reports ──────────────────────────────────────────────────────────────

/// Respons laporan/aduan.
#[derive(Debug, Serialize, ToSchema)]
pub struct ReportDocResponse {
    pub id: uuid::Uuid,
    pub reporter_id: uuid::Uuid,
    pub target_type: String,
    pub target_id: uuid::Uuid,
    pub keterangan: String,
    pub evidence_object_key: Option<String>,
    pub status: String,
    pub action_note: Option<String>,
    pub reviewed_by: Option<uuid::Uuid>,
    pub created_at: String,
    pub updated_at: String,
}

/// Detail aduan dengan presigned read URL bukti.
#[derive(Debug, Serialize, ToSchema)]
pub struct ReportDetailDocResponse {
    pub id: uuid::Uuid,
    pub reporter_id: uuid::Uuid,
    pub target_type: String,
    pub target_id: uuid::Uuid,
    pub keterangan: String,
    pub evidence_object_key: Option<String>,
    pub evidence_read_url: Option<String>,
    pub status: String,
    pub action_note: Option<String>,
    pub reviewed_by: Option<uuid::Uuid>,
    pub created_at: String,
    pub updated_at: String,
}

/// Payload pembuatan aduan dari mobile.
#[allow(dead_code)]
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateReportDocRequest {
    #[schema(example = "iklan")]
    pub target_type: String,
    pub target_id: uuid::Uuid,
    #[schema(example = "Iklan ini mengandung informasi palsu")]
    pub keterangan: String,
    pub mime: Option<String>,
    pub size_bytes: Option<u64>,
}

/// Payload tindak lanjut aduan oleh admin.
#[allow(dead_code)]
#[derive(Debug, Deserialize, ToSchema)]
pub struct ReviewReportDocRequest {
    #[schema(example = true)]
    pub approved: bool,
    #[schema(example = "Iklan telah di-suspend. Pengguna diberi peringatan.")]
    pub action_note: String,
}

/// POST /api/v1/reports — pengguna melaporkan iklan/user (mobile).
#[utoipa::path(post, path = "/api/v1/reports", tag = "content-reports",
    request_body = CreateReportDocRequest,
    responses(
        (status = 201, description = "Aduan dibuat; menunggu tinjauan admin", body = ReportDocResponse),
        (status = 422, description = "Validasi gagal (keterangan kosong, target_type invalid)")
    )
)]
#[allow(dead_code)]
fn create_report_doc() {}

/// GET /api/v1/reports/admin — admin listing aduan (search/filter/sort/pagination).
#[utoipa::path(get, path = "/api/v1/reports/admin", tag = "admin-reports",
    params(
        ("q" = Option<String>, Query, description = "Pencarian (ID pengguna / ID aduan)"),
        ("status" = Option<String>, Query, description = "Filter status (pending|in_review|rejected|resolved)"),
        ("sort_dir" = Option<String>, Query, description = "Arah sort (asc/desc, default desc)"),
        ("limit" = Option<i64>, Query, description = "Items per halaman (default 20)"),
        ("offset" = Option<i64>, Query, description = "Offset halaman (default 0)"),
    ),
    responses((status = 200, description = "Daftar aduan terpaginasi", body = Vec<ReportDocResponse>))
)]
#[allow(dead_code)]
fn admin_list_reports_doc() {}

/// GET /api/v1/reports/admin/{id} — detail aduan + presigned read URL bukti.
#[utoipa::path(get, path = "/api/v1/reports/admin/{id}", tag = "admin-reports",
    params(("id" = uuid::Uuid, Path, description = "ID aduan")),
    responses(
        (status = 200, description = "Detail aduan dengan URL bukti", body = ReportDetailDocResponse),
        (status = 404, description = "Aduan tidak ditemukan")
    )
)]
#[allow(dead_code)]
fn admin_get_report_doc() {}

/// POST /api/v1/reports/admin/{id}/review — admin menindaklanjuti aduan.
#[utoipa::path(post, path = "/api/v1/reports/admin/{id}/review", tag = "admin-reports",
    params(("id" = uuid::Uuid, Path, description = "ID aduan")),
    request_body = ReviewReportDocRequest,
    responses(
        (status = 200, description = "Aduan ditindaklanjuti; notifikasi dikirim ke pelapor", body = ReportDocResponse),
        (status = 404, description = "Aduan tidak ditemukan"),
        (status = 409, description = "Aduan sudah ditindaklanjuti (status terminal)"),
        (status = 422, description = "action_note wajib diisi")
    )
)]
#[allow(dead_code)]
fn admin_review_report_doc() {}

/// GET /api/v1/reports/admin/export.csv — ekspor CSV daftar aduan.
#[utoipa::path(get, path = "/api/v1/reports/admin/export.csv", tag = "admin-reports",
    params(
        ("q" = Option<String>, Query),
        ("status" = Option<String>, Query),
    ),
    responses((status = 200, description = "File CSV", content_type = "text/csv"))
)]
#[allow(dead_code)]
fn admin_export_reports_doc() {}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Rejki Backend API",
        description = "Dokumentasi API interaktif Rejki — hanya tersedia di environment development.",
        version = env!("CARGO_PKG_VERSION"),
    ),
    paths(
        // System
        health_doc,
        // Auth
        register_doc, login_doc, admin_login_doc, verify_otp_doc, resend_otp_doc,
        refresh_doc, logout_doc, forgot_password_doc, reset_password_doc,
        change_password_doc, suspend_doc, suspend_bulk_doc, suspend_evidence_doc,
        // Users / KYC
        get_me_doc, update_me_doc, submit_kyc_doc, kyc_status_doc,
        avatar_doc, request_doc_upload_doc, commit_document_doc, doc_read_doc,
        review_kyc_doc, get_by_id_doc,
        admin_list_kyc_doc, admin_export_kyc_doc, admin_get_kyc_doc, admin_get_document_doc,
        // Regions
        provinces_doc, regencies_doc, districts_doc, villages_doc,
        // Notifications
        notif_doc,
        // Iklan — public
        list_pekerjaan_doc, get_pekerjaan_doc, create_pekerjaan_doc,
        // Iklan — admin
        admin_list_pekerjaan_doc, export_csv_pekerjaan_doc,
        suspend_evidence_pekerjaan_doc, suspend_pekerjaan_doc,
        // Pelatihan — public
        list_pelatihan_doc, get_pelatihan_doc, create_pelatihan_doc,
        enroll_pelatihan_doc, badge_pelatihan_doc,
        // Pelatihan — admin
        admin_list_pelatihan_doc, admin_create_pelatihan_doc,
        admin_update_pelatihan_doc, admin_cancel_pelatihan_doc,
        admin_review_pelatihan_doc, admin_export_pelatihan_doc,
        admin_list_enrollment_doc, admin_detail_enrollment_doc,
        admin_review_enrollment_doc, admin_export_enrollment_doc,
        admin_list_badge_doc, admin_detail_badge_doc,
        admin_review_badge_doc, admin_export_badge_doc,
        // Corporate Comms — admin
        admin_list_articles_doc, admin_create_article_doc,
        article_photo_upload_doc,
        admin_get_article_doc, admin_update_article_doc,
        admin_delete_article_doc,
        // Content Reports
        create_report_doc, admin_list_reports_doc,
        admin_get_report_doc, admin_review_report_doc,
        admin_export_reports_doc,
    ),
    components(schemas(
        // Auth
        RegisterDocRequest, LoginDocRequest, AdminLoginDocRequest, LoginDocResponse,
        VerifyOtpDocRequest, ResendOtpDocRequest, RefreshDocRequest,
        ForgotPasswordDocRequest, ResetPasswordDocRequest, ChangePasswordDocRequest,
        SuspendDocRequest, BulkSuspendDocRequest,
        BulkSuspendItemDocResponse, BulkSuspendDocResponse, SuspendEvidenceDocRequest,
        // Users / KYC
        UserProfileDocResponse, UpdateProfileDocRequest, KycSubmitDocRequest,
        KycSubmissionDocResponse, UploadRequestDocRequest, CommitDocumentDocRequest,
        UploadPermissionDocResponse, ReviewKycDocRequest,
        AdminKycListItemDocResponse, AdminKycDetailDocResponse,
        // Regions
        RegionDocItem,
        // Notifications
        NotifItemDocResponse,
        // Iklan
        IklanPekerjaanDocResponse, AdminIklanDocResponse,
        CreateIklanDocRequest, SuspendIklanDocRequest,
        SuspendIklanDocResponse, SuspendIklanItemDocResponse,
        // Pelatihan
        IklanPelatihanDocResponse, AdminPelatihanDocResponse,
        CreatePelatihanDocRequest, UpdatePelatihanDocRequest,
        ReviewPelatihanDocRequest,
        EnrollmentDocResponse, ReviewEnrollmentDocRequest,
        BadgeDocResponse, ReviewBadgeDocRequest,
        PelatihanEvidenceDocRequest,
        // Corporate Comms
        AdminArticleDocResponse, CreateArticleDocRequest,
        UpdateArticleDocRequest, ArticlePhotoDocRequest,
        // Content Reports
        ReportDocResponse, ReportDetailDocResponse,
        CreateReportDocRequest, ReviewReportDocRequest,
    )),
    tags(
        (name = "system", description = "Health & operasional"),
        (name = "auth", description = "Autentikasi, OTP, & sesi"),
        (name = "users", description = "Profil pengguna"),
        (name = "kyc", description = "Onboarding KYC: data diri & dokumen"),
        (name = "admin", description = "Tinjau KYC, suspend akun"),
        (name = "regions", description = "Data wilayah Indonesia (cascading)"),
        (name = "notifications", description = "Notifikasi in-app pengguna"),
        (name = "iklan-pekerjaan", description = "Iklan lowongan pekerjaan (publik)"),
        (name = "admin-iklan", description = "Moderasi iklan — listing, suspend, CSV export (semua vertikal)"),
        (name = "pelatihan", description = "Pelatihan — listing publik, pendaftaran (enroll), & pengajuan badge"),
        (name = "admin-pelatihan", description = "Admin pelatihan — moderasi, enrollment review, badge review, CSV export"),
        (name = "admin-corporate-comms", description = "Admin Corporate Communication — artikel, foto, broadcast notifikasi"),
        (name = "content-reports", description = "Pelaporan konten — pengguna melaporkan iklan/pengguna (mobile)"),
        (name = "admin-reports", description = "Admin Content Reports — listing, detail, tindak lanjut, CSV export aduan")
    )
)]
pub struct ApiDoc;
