use auth_service::TokenIssuer;
use sqlx::PgPool;
use uuid::Uuid;

/// Generate access token untuk executive (rank 90).
pub async fn executive_token(pool: &PgPool, suffix: &str) -> String {
    token_for_role(pool, suffix, "executive", auth_service::Role::Executive).await
}

/// Generate access token untuk super_admin (rank 100).
pub async fn super_admin_token(pool: &PgPool, suffix: &str) -> String {
    token_for_role(pool, suffix, "super_admin", auth_service::Role::SuperAdmin).await
}

/// Generate access token untuk user biasa (rank 20).
pub async fn user_token(pool: &PgPool, suffix: &str) -> String {
    token_for_role(pool, suffix, "user", auth_service::Role::User).await
}

/// Internal: seed user di auth.users dan generate JWT.
async fn token_for_role(pool: &PgPool, suffix: &str, role_str: &str, role: auth_service::Role) -> String {
    let id = Uuid::now_v7();
    let email = format!("ceo-test-{suffix}@test.rejki.internal");
    let pw_hash = bcrypt::hash("Test1234!", 4).expect("bcrypt failed");

    sqlx::query(
        r#"
        INSERT INTO auth.users (id, email, password_hash, status, role, created_at, updated_at)
        VALUES ($1, $2, $3, 'active', $4, now(), now())
        ON CONFLICT (email) DO UPDATE
            SET status = 'active', role = $4
        "#,
    )
    .bind(id)
    .bind(&email)
    .bind(&pw_hash)
    .bind(role_str)
    .execute(pool)
    .await
    .expect("token_for_role: insert user failed");

    // Seed profile minimal
    sqlx::query(
        r#"
        INSERT INTO user_svc.profiles (id, auth_id, username, created_at, updated_at)
        VALUES ($1, $1, $2, now(), now())
        ON CONFLICT (auth_id) DO NOTHING
        "#,
    )
    .bind(id)
    .bind(format!("ceo_test_{suffix}"))
    .execute(pool)
    .await
    .expect("token_for_role: insert profile failed");

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

    jwt.issue_access_token(
        id,
        &email,
        auth_service::AccountStatus::Active,
        role,
    )
    .expect("issue_access_token failed")
}

/// Hapus data test (email domain @test.rejki.internal).
pub async fn clean_test_data(pool: &PgPool) {
    sqlx::query(
        "DELETE FROM chat.messages WHERE conversation_id IN \
         (SELECT id FROM chat.conversations WHERE id IN \
          (SELECT id FROM auth.users WHERE email LIKE '%@test.rejki.internal'::text))",
    )
    .execute(pool)
    .await
    .ok();

    sqlx::query(
        "DELETE FROM user_svc.kyc_submissions WHERE profile_id IN \
         (SELECT id FROM user_svc.profiles WHERE auth_id IN \
          (SELECT id FROM auth.users WHERE email LIKE '%@test.rejki.internal'))",
    )
    .execute(pool)
    .await
    .expect("clean: kyc failed");

    sqlx::query(
        "DELETE FROM user_svc.profiles WHERE auth_id IN \
         (SELECT id FROM auth.users WHERE email LIKE '%@test.rejki.internal')",
    )
    .execute(pool)
    .await
    .expect("clean: profiles failed");

    sqlx::query(
        "DELETE FROM auth.users WHERE email LIKE '%@test.rejki.internal'",
    )
    .execute(pool)
    .await
    .expect("clean: users failed");
}
