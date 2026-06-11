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
        .issue_access_token(id, &email, auth_service_client::AccountStatus::Active)
        .expect("issue_access_token failed");

    TestUser {
        id,
        email,
        access_token,
    }
}

/// Hapus semua data test (semua row dengan email domain @test.rejki.internal).
/// Dipanggil di awal setiap test suite untuk idempotency, bukan di teardown
/// (teardown yang crash meninggalkan data kotor).
pub async fn clean_test_data(pool: &PgPool) {
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
