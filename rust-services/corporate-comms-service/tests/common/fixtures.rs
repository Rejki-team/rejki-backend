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

/// Seed satu user biasa ke auth.users dan kembalikan TestUser dengan valid JWT.
///
/// Password di-hash dengan bcrypt cost=4 (minimum — cepat untuk test).
/// User langsung di-set status `active` dan role `user`.
pub async fn seed_user(pool: &PgPool, suffix: &str) -> TestUser {
    let id = Uuid::now_v7();
    let email = format!("test-{suffix}@test.rejki.internal");

    let pw_hash = bcrypt::hash("Test1234!", 4).expect("bcrypt failed");

    sqlx::query(
        r#"
        INSERT INTO auth.users (id, email, password_hash, status, role, created_at, updated_at)
        VALUES ($1, $2, $3, 'active', 'user', now(), now())
        ON CONFLICT (email) DO UPDATE
            SET status = 'active'
        RETURNING id
        "#,
    )
    .bind(id)
    .bind(&email)
    .bind(&pw_hash)
    .fetch_one(pool)
    .await
    .expect("seed_user: insert failed");

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

/// Seed satu admin (role=super_admin, status=active) + profil, kembalikan TestUser
/// dengan JWT ber-klaim role=super_admin. Dipakai untuk menguji endpoint admin.
pub async fn seed_admin(pool: &PgPool, suffix: &str) -> TestUser {
    let id = Uuid::now_v7();
    let email = format!("admin-{suffix}@test.rejki.internal");
    let pw_hash = bcrypt::hash("Test1234!", 4).expect("bcrypt failed");

    sqlx::query(
        r#"
        INSERT INTO auth.users (id, email, password_hash, status, role, created_at, updated_at)
        VALUES ($1, $2, $3, 'active', 'super_admin', now(), now())
        ON CONFLICT (email) DO UPDATE
            SET status = 'active', role = 'super_admin'
        RETURNING id
        "#,
    )
    .bind(id)
    .bind(&email)
    .bind(&pw_hash)
    .fetch_one(pool)
    .await
    .expect("seed_admin: insert failed");

    sqlx::query(
        r#"
        INSERT INTO user_svc.profiles (id, auth_id, username, created_at, updated_at)
        VALUES ($1, $1, $2, now(), now())
        ON CONFLICT (auth_id) DO NOTHING
        "#,
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

/// Seed sebuah artikel ke tabel `comms.corporate_article` milik `author_id`.
/// Mengembalikan UUID dari artikel yang dibuat.
pub async fn seed_article(pool: &PgPool, author_id: Uuid, suffix: &str) -> Uuid {
    let id = Uuid::now_v7();

    sqlx::query(
        r#"
        INSERT INTO comms.corporate_article
            (id, author_id, category, title, body, deleted_at, created_at, updated_at)
        VALUES
            ($1, $2, 'informasi', $3, $4, NULL, now(), now())
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(id)
    .bind(author_id)
    .bind(format!("Artikel {suffix}"))
    .bind(format!("Isi artikel {suffix}"))
    .execute(pool)
    .await
    .expect("seed_article: insert failed");

    id
}

/// Hapus semua data test (semua row dengan email domain @test.rejki.internal).
/// Dipanggil di awal setiap test suite untuk idempotency, bukan di teardown
/// (teardown yang crash meninggalkan data kotor).
pub async fn clean_test_data(pool: &PgPool) {
    // Hapus artikel milik user test
    sqlx::query(
        "DELETE FROM comms.corporate_article WHERE author_id IN \
         (SELECT id FROM auth.users WHERE email LIKE '%@test.rejki.internal')",
    )
    .execute(pool)
    .await
    .expect("clean_test_data: articles failed");

    // Hapus profil yang auth_id-nya mengarah ke user test, lalu user test
    sqlx::query(
        "DELETE FROM user_svc.profiles WHERE auth_id IN \
         (SELECT id FROM auth.users WHERE email LIKE '%@test.rejki.internal')",
    )
    .execute(pool)
    .await
    .expect("clean_test_data: profiles failed");

    sqlx::query("DELETE FROM auth.users WHERE email LIKE '%@test.rejki.internal'")
        .execute(pool)
        .await
        .expect("clean_test_data: users failed");
}
