use auth_service::TokenIssuer;
use sqlx::PgPool;
use uuid::Uuid;

/// Representasi user yang sudah di-seed ke DB test.
/// Email menggunakan domain @test.rejki.internal agar tidak bocor ke prod.
pub struct TestUser {
    pub id: Uuid,
    /// Disimpan untuk kelengkapan seed; belum dibaca di assertion saat ini.
    #[allow(dead_code)]
    pub email: String,
    pub access_token: String,
}

/// Seed satu user ke auth.users dan kembalikan TestUser dengan valid JWT.
///
/// Password di-hash dengan bcrypt cost=4 (minimum — cepat untuk test).
/// OTP skip: user langsung di-set status `active` (boleh memakai fitur).
pub async fn seed_user(pool: &PgPool, suffix: &str) -> TestUser {
    let id = Uuid::now_v7();
    let email = format!("test-{suffix}@test.rejki.internal");

    // Hash password "Test1234!" — cost 4 agar test cepat
    let pw_hash = bcrypt::hash("Test1234!", 4).expect("bcrypt failed");

    sqlx::query!(
        r#"
        INSERT INTO auth.users (id, email, password_hash, status, created_at, updated_at)
        VALUES ($1, $2, $3, 'active', now(), now())
        ON CONFLICT (email) DO UPDATE
            SET status = 'active'
        RETURNING id
        "#,
        id,
        email,
        pw_hash,
    )
    .fetch_one(pool)
    .await
    .expect("seed_user: insert failed");

    // Seed profil minimal di user_svc agar GET /users/me mengembalikan 200.
    // username unik per suffix untuk menghindari konflik.
    // id = auth_id agar konsisten dgn get_profile yang saat ini mencari by profile id.
    sqlx::query!(
        r#"
        INSERT INTO user_svc.profiles (id, auth_id, username, created_at, updated_at)
        VALUES ($1, $1, $2, now(), now())
        ON CONFLICT (auth_id) DO NOTHING
        "#,
        id,
        format!("user_{suffix}"),
    )
    .execute(pool)
    .await
    .expect("seed_user: insert profile failed");

    // Buat JWT langsung — tidak perlu hit /login untuk menghindari OTP flow
    let private_pem = std::fs::read_to_string(
        std::env::var("JWT_PRIVATE_KEY_PATH").unwrap_or_else(|_| "./keys/private.pem".into()),
    )
    .expect("JWT_PRIVATE_KEY_PATH not found");

    let public_pem = std::fs::read_to_string(
        std::env::var("JWT_PUBLIC_KEY_PATH").unwrap_or_else(|_| "./keys/public.pem".into()),
    )
    .expect("JWT_PUBLIC_KEY_PATH not found");

    let jwt = auth_service::JwtService::from_files(&private_pem, &public_pem, 900)
        .expect("JwtService init failed");

    let access_token = jwt
        .issue_access_token(
            id,
            &email,
            auth_service::AccountStatus::Active,
            auth_service::Role::User,
        )
        .expect("issue_access_token failed");

    TestUser {
        id,
        email,
        access_token,
    }
}

/// Seed satu admin (role=admin, status active) + profil, kembalikan TestUser dengan
/// JWT ber-klaim role=admin. Dipakai untuk menguji endpoint `/admin/**`.
#[allow(dead_code)]
pub async fn seed_admin(pool: &PgPool, suffix: &str) -> TestUser {
    let id = Uuid::now_v7();
    let email = format!("admin-{suffix}@test.rejki.internal");
    let pw_hash = bcrypt::hash("Test1234!", 4).expect("bcrypt failed");

    sqlx::query!(
        r#"
        INSERT INTO auth.users (id, email, password_hash, status, role, created_at, updated_at)
        VALUES ($1, $2, $3, 'active', 'super_admin', now(), now())
        ON CONFLICT (email) DO UPDATE
            SET status = 'active', role = 'super_admin'
        RETURNING id
        "#,
        id,
        email,
        pw_hash,
    )
    .fetch_one(pool)
    .await
    .expect("seed_admin: insert failed");

    sqlx::query!(
        r#"
        INSERT INTO user_svc.profiles (id, auth_id, username, created_at, updated_at)
        VALUES ($1, $1, $2, now(), now())
        ON CONFLICT (auth_id) DO NOTHING
        "#,
        id,
        format!("admin_{suffix}"),
    )
    .execute(pool)
    .await
    .expect("seed_admin: insert profile failed");

    let private_pem = std::fs::read_to_string(
        std::env::var("JWT_PRIVATE_KEY_PATH").unwrap_or_else(|_| "./keys/private.pem".into()),
    )
    .expect("JWT_PRIVATE_KEY_PATH not found");
    let public_pem = std::fs::read_to_string(
        std::env::var("JWT_PUBLIC_KEY_PATH").unwrap_or_else(|_| "./keys/public.pem".into()),
    )
    .expect("JWT_PUBLIC_KEY_PATH not found");
    let jwt = auth_service::JwtService::from_files(&private_pem, &public_pem, 900)
        .expect("JwtService init failed");

    let access_token = jwt
        .issue_access_token(
            id,
            &email,
            auth_service::AccountStatus::Active,
            auth_service::Role::SuperAdmin,
        )
        .expect("issue_access_token failed");

    TestUser {
        id,
        email,
        access_token,
    }
}

/// Seed satu pengajuan KYC (profil + data diri + submission) untuk user test.
/// Mengembalikan `submission_id`. `status` ∈ {pending, approved, rejected}.
/// `with_documents` mengisi object key KTP/Selfie agar uji akses dokumen/purge.
#[allow(dead_code)]
pub async fn seed_kyc_submission(
    pool: &PgPool,
    target: &TestUser,
    status: &str,
    with_documents: bool,
) -> Uuid {
    // Selaraskan status akun dengan tahap KYC agar transisi review valid
    // (state machine auth: PendingKyc → Active/Rejected). Untuk submission terminal,
    // status akun mengikuti hasil agar pra-kondisi konsisten.
    let account_status = match status {
        "approved" => "active",
        "rejected" => "rejected",
        _ => "pending_kyc",
    };
    sqlx::query!(
        "UPDATE auth.users SET status = $2, updated_at = now() WHERE id = $1",
        target.id,
        account_status,
    )
    .execute(pool)
    .await
    .expect("seed_kyc_submission: update account status failed");

    // Lengkapi profil dengan data diri (NIK ter-mask via nik_last4) untuk listing.
    sqlx::query!(
        r#"
        UPDATE user_svc.profiles
        SET full_name = 'Budi Tester', nik_last4 = '8901',
            education_level = 's1', gender = 'male',
            birth_date = DATE '1995-05-05', address_line = 'Jl. Uji No. 1',
            country_code = 'ID', province_id = '11', regency_id = '1101',
            district_id = '110101', village_id = '1101012001', updated_at = now()
        WHERE id = $1
        "#,
        target.id,
    )
    .execute(pool)
    .await
    .expect("seed_kyc_submission: update profile failed");

    let (ktp, selfie): (Option<String>, Option<String>) = if with_documents {
        (
            Some(format!("uploads/ktp/{}/ktp.jpg", target.id)),
            Some(format!("uploads/selfie/{}/selfie.jpg", target.id)),
        )
    } else {
        (None, None)
    };

    let row = sqlx::query!(
        r#"
        INSERT INTO user_svc.kyc_submission
            (id, profile_id, status, ktp_object_key, selfie_object_key, created_at, updated_at)
        VALUES (gen_random_uuid(), $1, $2, $3, $4, now(), now())
        RETURNING id
        "#,
        target.id,
        status,
        ktp,
        selfie,
    )
    .fetch_one(pool)
    .await
    .expect("seed_kyc_submission: insert submission failed");

    row.id
}

/// Hapus semua data test (semua row dengan email domain @test.rejki.internal).
/// Dipanggil di awal setiap test suite untuk idempotency, bukan di teardown
/// (teardown yang crash meninggalkan data kotor).
pub async fn clean_test_data(pool: &PgPool) {
    // Submission KYC milik profil test (hindari baris yatim setelah profil dihapus).
    sqlx::query!(
        "DELETE FROM user_svc.kyc_submission WHERE profile_id IN \
         (SELECT id FROM user_svc.profiles WHERE auth_id IN \
          (SELECT id FROM auth.users WHERE email LIKE '%@test.rejki.internal'))"
    )
    .execute(pool)
    .await
    .expect("clean_test_data: kyc_submission failed");

    // Audit akses dokumen oleh aktor test.
    sqlx::query!(
        "DELETE FROM user_svc.document_access_log WHERE actor_id IN \
         (SELECT id FROM auth.users WHERE email LIKE '%@test.rejki.internal')"
    )
    .execute(pool)
    .await
    .expect("clean_test_data: document_access_log failed");

    // Hapus profil yang auth_id-nya mengarah ke user test, lalu user test.
    sqlx::query!(
        "DELETE FROM user_svc.profiles WHERE auth_id IN \
         (SELECT id FROM auth.users WHERE email LIKE '%@test.rejki.internal')"
    )
    .execute(pool)
    .await
    .expect("clean_test_data: profiles failed");

    sqlx::query!("DELETE FROM auth.users WHERE email LIKE '%@test.rejki.internal'")
        .execute(pool)
        .await
        .expect("clean_test_data failed");
}
