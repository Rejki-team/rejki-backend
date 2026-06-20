use auth_service::TokenIssuer;
use sqlx::PgPool;
use uuid::Uuid;

pub struct TestUser {
    pub id: Uuid,
    #[allow(dead_code)]
    pub email: String,
    pub access_token: String,
}

pub async fn seed_user(pool: &PgPool, suffix: &str) -> TestUser {
    let id = Uuid::now_v7();
    let email = format!("test-{suffix}@test.rejki.internal");
    let pw_hash = bcrypt::hash("Test1234!", 4).expect("bcrypt failed");

    sqlx::query(
        "INSERT INTO auth.users (id, email, password_hash, status, created_at, updated_at)
         VALUES ($1, $2, $3, 'active', now(), now())
         ON CONFLICT (email) DO UPDATE SET status = 'active'
         RETURNING id",
    )
    .bind(id)
    .bind(&email)
    .bind(&pw_hash)
    .execute(pool)
    .await
    .expect("seed_user: insert failed");

    sqlx::query(
        "INSERT INTO user_svc.profiles (id, auth_id, username, created_at, updated_at)
         VALUES ($1, $1, $2, now(), now())
         ON CONFLICT (auth_id) DO NOTHING",
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

pub async fn seed_admin(pool: &PgPool, suffix: &str) -> TestUser {
    let id = Uuid::now_v7();
    let email = format!("admin-{suffix}@test.rejki.internal");
    let pw_hash = bcrypt::hash("Test1234!", 4).expect("bcrypt failed");

    sqlx::query(
        "INSERT INTO auth.users (id, email, password_hash, status, role, created_at, updated_at)
         VALUES ($1, $2, $3, 'active', 'super_admin', now(), now())
         ON CONFLICT (email) DO UPDATE SET status = 'active', role = 'super_admin'
         RETURNING id",
    )
    .bind(id)
    .bind(&email)
    .bind(&pw_hash)
    .execute(pool)
    .await
    .expect("seed_admin: insert failed");

    sqlx::query(
        "INSERT INTO user_svc.profiles (id, auth_id, username, created_at, updated_at)
         VALUES ($1, $1, $2, now(), now())
         ON CONFLICT (auth_id) DO NOTHING",
    )
    .bind(id)
    .bind(format!("admin_{suffix}"))
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

pub async fn clean_test_data(pool: &PgPool) {
    sqlx::query(
        "DELETE FROM iklan_pekerja.iklan_suspension WHERE iklan_id IN \
         (SELECT id FROM iklan_pekerja.iklan WHERE poster_id IN \
          (SELECT id FROM auth.users WHERE email LIKE '%@test.rejki.internal'))",
    )
    .execute(pool)
    .await
    .ok();

    sqlx::query(
        "DELETE FROM iklan_pekerja.iklan WHERE poster_id IN \
         (SELECT id FROM auth.users WHERE email LIKE '%@test.rejki.internal')",
    )
    .execute(pool)
    .await
    .ok();

    sqlx::query(
        "DELETE FROM user_svc.profiles WHERE auth_id IN \
         (SELECT id FROM auth.users WHERE email LIKE '%@test.rejki.internal')",
    )
    .execute(pool)
    .await
    .ok();

    sqlx::query("DELETE FROM auth.users WHERE email LIKE '%@test.rejki.internal'")
        .execute(pool)
        .await
        .ok();
}
