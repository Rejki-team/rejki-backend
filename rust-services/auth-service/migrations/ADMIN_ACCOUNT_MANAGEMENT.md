# Panduan Operasional — Akun Admin Rejki

**Ref:** openspec/changes/add-admin-rbac D5
**Audiens:** DBA / DevOps / Operations

## 1. Membuat Akun Admin Baru

Akun admin TIDAK dibuat melalui endpoint publik. Akun hanya dibuat via SQL langsung oleh DBA.

### Langkah-langkah:

1. **Generate password hash** menggunakan bcrypt (cost 12):

   ```bash
   # Via Python
   python -c "import bcrypt; print(bcrypt.hashpw(b'<YOUR_PASSWORD>'.encode(), bcrypt.gensalt(12)).decode())"
   ```

   Atau via CLI helper (jika tersedia):
   ```bash
   cargo run --bin seed-admin -- --email admin@rejki.id --password <PASSWORD>
   ```

2. **Insert ke database** (ganti `<HASH>` dengan hash dari langkah 1):

   ```sql
   INSERT INTO auth.users (id, email, password_hash, status, role, tos_accepted_at, tos_version)
   VALUES (
       gen_random_uuid(),
       'admin-baru@rejki.id',
       '<HASH>',
       'active',
       'admin',
       now(),
       'v1'
   );
   ```

3. **Verifikasi**: login via `POST /api/v1/auth/admin/login` dengan email + password.

## 2. Merotasi Password Admin

Gunakan alur reset password yang sama seperti user reguler:

### Opsi A — Via endpoint (self-service untuk admin):

```bash
# 1. Minta OTP reset password
curl -X POST https://api.rejki.id/api/v1/auth/forgot-password \
  -H "Content-Type: application/json" \
  -d '{"email": "admin@rejki.id"}'

# 2. Submit password baru dengan OTP
curl -X POST https://api.rejki.id/api/v1/auth/reset-password \
  -H "Content-Type: application/json" \
  -d '{"email": "admin@rejki.id", "otp": "123456", "new_password": "NewPass123!"}'
```

Ini mencabut **semua** refresh token (semua sesi login lama).

### Opsi B — Via DBA (direct SQL, darurat):

```sql
UPDATE auth.users 
SET password_hash = '<NEW_BCRYPT_HASH>', updated_at = now() 
WHERE email = 'admin@rejki.id';
```

### Opsi C — Via commit SQL (via PR, review, deployment):

1. Generate hash baru.
2. Buat PR ke repo dengan update `migrations/...seed_admin_account.up.sql`.
3. Review & merge.
4. Deploy migrasi.

## 3. Menonaktifkan Akun Admin

```sql
-- Suspend sementara (auto-pulih setelah expires_at):
UPDATE auth.users SET status = 'suspended_temp', updated_at = now() WHERE email = '<admin_email>';
INSERT INTO auth.account_suspension (id, user_id, permanent, reason, expires_at, created_by)
VALUES (gen_random_uuid(), '<admin_user_id>', false, 'rotasi akun', now() + interval '24 hours', '<admin_user_id>');

-- Suspend permanen:
UPDATE auth.users SET status = 'suspended_permanent', updated_at = now() WHERE email = '<admin_email>';
```

## 4. Menambah Role Baru di Masa Depan

Role dirancang **extensible**. Untuk menambah role baru (mis. `super_admin`, `moderator`):

1. Update CHECK constraint di database (migrasi UP):
   ```sql
   ALTER TABLE auth.users DROP CONSTRAINT auth_users_role_check;
   ALTER TABLE auth.users ADD CONSTRAINT auth_users_role_check
       CHECK (role IN ('user', 'admin', 'super_admin', 'moderator'));
   ```

2. Tambah varian pada enum `Role` di `auth-service-client/src/lib.rs`:
   ```rust
   #[non_exhaustive]
   pub enum Role {
       User,
       Admin,
       SuperAdmin,
       Moderator,
   }
   ```

3. Update logic `is_admin()` atau buat permission matrix sesuai kebutuhan.
4. Update middleware (mis. `require_role(Role::Moderator)`) di `common/auth-middleware`.

## 5. Catatan Keamanan

- **Kredensial tidak boleh di-log.** Tracing hanya mencatat `user_id` + `email` — tidak pernah password atau token.
- **Rotasi password** setiap 90 hari (sesuai kebijakan keamanan organisasi).
- **Gunakan password kuat** (minimal 12 karakter, campuran huruf besar/kecil/angka/simbol).
- **Jangan hardcode password di kode**. Gunakan environment variable atau secrets manager.
- Backend tidak membocorkan keberadaan akun (anti-enumeration) pada endpoint login admin.
