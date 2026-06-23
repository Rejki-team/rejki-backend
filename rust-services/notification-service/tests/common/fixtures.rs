use auth_service::TokenIssuer;
use sqlx::PgPool;
use uuid::Uuid;

/// Representasi user yang sudah di-seed ke DB test.
pub struct TestUser {
    pub id: Uuid,
    #[allow(dead_code)]
    pub email: String,
    pub access_token: String,
}

/// Seed satu user ke auth.users dan kembalikan TestUser dengan valid JWT.
pub async fn seed_user(pool: &PgPool, suffix: &str) -> TestUser {
    let id = Uuid::now_v7();
    let email = format!("test-{suffix}@test.rejki.internal");

    let pw_hash = bcrypt::hash("Test1234!", 4).expect("bcrypt failed");

    sqlx::query(
        r#"
        INSERT INTO auth.users (id, email, password_hash, status, created_at, updated_at)
        VALUES ($1, $2, $3, 'active', now(), now())
        ON CONFLICT (email) DO UPDATE
            SET status = 'active'
        "#,
    )
    .bind(id)
    .bind(&email)
    .bind(&pw_hash)
    .execute(pool)
    .await
    .expect("seed_user: insert failed");

    // Seed profil minimal di user_svc
    sqlx::query(
        r#"
        INSERT INTO user_svc.profiles (id, auth_id, username, created_at, updated_at)
        VALUES ($1, $1, $2, now(), now())
        ON CONFLICT (auth_id) DO NOTHING
        "#,
    )
    .bind(id)
    .bind(format!("user_{suffix}"))
    .execute(pool)
    .await
    .expect("seed_user: insert profile failed");

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

/// Seed satu notifikasi ke tabel `notification.notifications` milik `recipient_id`.
/// Mengembalikan UUID dari notifikasi yang dibuat.
pub async fn seed_notification(pool: &PgPool, recipient_id: Uuid, suffix: &str) -> Uuid {
    let id = Uuid::now_v7();

    sqlx::query(
        r#"
        INSERT INTO notification.notifications (id, recipient_id, title, body, is_read, created_at)
        VALUES ($1, $2, $3, $4, false, now())
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(id)
    .bind(recipient_id)
    .bind(format!("Notifikasi {suffix}"))
    .bind(format!("Body notifikasi {suffix}"))
    .execute(pool)
    .await
    .expect("seed_notification: insert failed");

    id
}

/// Hapus semua data test (semua row dengan email domain @test.rejki.internal).
pub async fn clean_test_data(pool: &PgPool) {
    // Hapus notifikasi milik user test
    sqlx::query(
        "DELETE FROM notification.notifications WHERE recipient_id IN \
         (SELECT id FROM auth.users WHERE email LIKE '%@test.rejki.internal')",
    )
    .execute(pool)
    .await
    .expect("clean_test_data: notifications failed");

    // Hapus profil
    sqlx::query(
        "DELETE FROM user_svc.profiles WHERE auth_id IN \
         (SELECT id FROM auth.users WHERE email LIKE '%@test.rejki.internal')",
    )
    .execute(pool)
    .await
    .expect("clean_test_data: profiles failed");

    // Hapus user test
    sqlx::query("DELETE FROM auth.users WHERE email LIKE '%@test.rejki.internal'")
        .execute(pool)
        .await
        .expect("clean_test_data: users failed");
}
