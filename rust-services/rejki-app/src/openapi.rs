//! Definisi OpenAPI untuk Swagger UI — HANYA disajikan saat `APP_ENV=development`
//! (lihat gate di `main.rs`). Dokumen ini sengaja minimal dan tumbuh inkremental.
//!
//! Catatan arsitektur (design D7): skema untuk dokumentasi didefinisikan sebagai
//! "mirror" lokal di composition root ini, BUKAN dengan menambah `utoipa::ToSchema`
//! ke DTO crate domain (mis. `auth-service`). Dengan begitu crate domain tidak
//! menarik dependency dokumentasi hanya demi Swagger.

use serde::{Deserialize, Serialize};
use utoipa::{OpenApi, ToSchema};

/// Mirror payload login (`POST /api/v1/auth/login`) — untuk dokumentasi saja.
/// Mereplikasi `auth_service::application::dto::LoginInput`.
/// Field hanya dibaca oleh `utoipa` untuk menurunkan skema, bukan runtime.
#[allow(dead_code)]
#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginDocRequest {
    /// Email pengguna.
    #[schema(example = "user@rejki.id")]
    pub email: String,
    /// Kata sandi (plaintext saat dikirim; selalu via HTTPS).
    #[schema(example = "rahasia123")]
    pub password: String,
}

/// Mirror respons token login — untuk dokumentasi saja.
/// Mereplikasi bentuk `data` pada respons sukses login.
#[derive(Debug, Serialize, ToSchema)]
pub struct LoginDocResponse {
    pub access_token: String,
    pub refresh_token: String,
    #[schema(example = "Bearer")]
    pub token_type: String,
    /// Masa berlaku access token dalam detik.
    #[schema(example = 900)]
    pub expires_in: u64,
}

/// Teladan anotasi endpoint health-check.
#[utoipa::path(
    get,
    path = "/health",
    tag = "system",
    responses(
        (status = 200, description = "Service sehat")
    )
)]
#[allow(dead_code)]
fn health_doc() {}

/// Teladan anotasi endpoint bisnis (design D7): `POST /api/v1/auth/login`.
#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    tag = "auth",
    request_body = LoginDocRequest,
    responses(
        (status = 200, description = "Login berhasil — kembalikan pasangan token", body = LoginDocResponse),
        (status = 422, description = "Kredensial tidak valid / input gagal validasi")
    )
)]
#[allow(dead_code)]
fn login_doc() {}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Rejki Backend API",
        description = "Dokumentasi API interaktif Rejki — hanya tersedia di environment development.",
        version = env!("CARGO_PKG_VERSION"),
    ),
    paths(health_doc, login_doc),
    components(schemas(LoginDocRequest, LoginDocResponse)),
    tags(
        (name = "system", description = "Health & operasional"),
        (name = "auth", description = "Autentikasi & sesi")
    )
)]
pub struct ApiDoc;
