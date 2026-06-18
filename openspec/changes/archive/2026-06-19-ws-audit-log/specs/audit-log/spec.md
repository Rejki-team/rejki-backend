# audit-log

## ADDED: Tabel `auth.audit_log`

### Schema

Tabel `auth.audit_log` — append-only, immutable:

| Kolom | Tipe | Constraint |
|---|---|---|
| `id` | UUID | PK, DEFAULT gen_random_uuid() |
| `user_id` | UUID | NULL untuk failed login |
| `event` | TEXT | NOT NULL |
| `ip_address` | INET | NULLable |
| `user_agent` | TEXT | NULLable |
| `request_id` | TEXT | NULLable |
| `metadata` | JSONB | NULLable |
| `created_at` | TIMESTAMPTZ | NOT NULL DEFAULT now() |

Index:
- `idx_audit_log_user_id` ON `auth.audit_log (user_id)`
- `idx_audit_log_created_at` ON `auth.audit_log (created_at)`

Revoke:
- `REVOKE UPDATE, DELETE ON auth.audit_log FROM rejki`

### SCENARIO: Insert audit event succeeds

GIVEN tabel `auth.audit_log` sudah dibuat
AND ada user terautentikasi
WHEN `PgAuditLogRepository::log(user_id, event, context)` dipanggil
THEN satu baris baru di-INSERT dengan semua field terisi
AND `id` di-generate otomatis (UUID)
AND `created_at` di-set otomatis (now)

### SCENARIO: Insert nil user_id untuk failed login

GIVEN user tidak ditemukan saat login
WHEN `log(None, LoginFailed { reason }, context)` dipanggil
THEN baris baru di-INSERT dengan `user_id = NULL`
AND `event = 'login_failed'`
AND `metadata` berisi `{ "reason": "..." }`

### SCENARIO: UPDATE/DELETE ditolak

GIVEN user DB `rejki` (aplikasi)
WHEN mencoba UPDATE atau DELETE dari `auth.audit_log`
THEN operasi ditolak oleh PostgreSQL (permission denied)

### SCENARIO: index user_id berfungsi

GIVEN banyak baris di `audit_log`
WHEN query `SELECT * FROM auth.audit_log WHERE user_id = $1`
THEN index `idx_audit_log_user_id` digunakan (index scan)

## ADDED: AuditLogRepository trait

### Trait signature

```rust
#[allow(async_fn_in_trait)]
pub trait AuditLogRepository: Send + Sync {
    async fn log(&self, user_id: Option<Uuid>, event: AuditEvent, context: AuditContext) -> Result<(), anyhow::Error>;
}
```

### SCENARIO: Trait implementable for mock

GIVEN MockAuditLogRepository implementing AuditLogRepository
WHEN `log()` dipanggil di unit test
THEN mock mencatat invocation tanpa side effect

## ADDED: AuditEvent enum

### Variants

| Variant | Metadata (JSONB) |
|---|---|
| `LoginSuccess` | — |
| `LoginFailed { reason: String }` | `{"reason": "..."}` |
| `AdminLoginSuccess` | — |
| `AdminLoginFailed { reason: String }` | `{"reason": "..."}` |
| `Logout` | — |
| `TokenRefresh` | — |
| `OtpSent { purpose: String }` | `{"purpose": "register|reset_password|change_password"}` |
| `PasswordChanged` | — |
| `PasswordReset` | — |
| `AccountSuspended { admin_id: Uuid, reason: String, permanent: bool }` | `{"admin_id": "...", "reason": "...", "permanent": true/false}` |
| `AccountSuspendedBulk { admin_id: Uuid, count: usize, permanent: bool }` | `{"admin_id": "...", "count": N, "permanent": true/false}` |

### SCENARIO: AuditEvent serializes to string

GIVEN AuditEvent::LoginFailed { reason: "wrong password".into() }
WHEN `event.as_str()` dipanggil
THEN return `"login_failed"`

### SCENARIO: All variants have distinct event strings

GIVEN semua 11+ variant AuditEvent
WHEN `as_str()` dipanggil di tiap variant
THEN setiap variant mengembalikan string unik

## ADDED: AuditContext struct

```rust
pub struct AuditContext {
    pub ip_address: Option<std::net::IpAddr>,
    pub user_agent: Option<String>,
    pub request_id: Option<String>,
}
```

### SCENARIO: AuditContext extracted from request headers

GIVEN HTTP request dengan header `x-forwarded-for`, `user-agent`, `x-request-id`
WHEN `AuditContext` di-extract via axum `FromRequestParts`
THEN `ip_address` = parsed dari `x-forwarded-for`
AND `user_agent` = header `user-agent`
AND `request_id` = header `x-request-id`

## ADDED: PgAuditLogRepository

### SCENARIO: Insert via PgAuditLogRepository

GIVEN PgAuditLogRepository dengan pool yang valid
WHEN `log(user_id, event, context)` dipanggil
THEN satu baris di-INSERT ke `auth.audit_log`
AND query selesai dalam < 100ms

## MODIFIED: AuthService injects AuditLogRepository

### SCENARIO: AuthService logs login_success

GIVEN AuthService dengan audit_log_repo yang di-inject
WHEN `login()` berhasil
THEN `audit_log_repo.log(Some(user.id), LoginSuccess, context)` dipanggil

### SCENARIO: AuthService logs login_failed

GIVEN AuthService dengan audit_log_repo
WHEN `login()` gagal karena password salah
THEN `audit_log_repo.log(None, LoginFailed { reason }, context)` dipanggil

### SCENARIO: AuthService fail-open on audit failure

GIVEN `audit_log_repo.log()` mengembalikan Err (simulasi DB down)
WHEN `login()` dipanggil dengan kredensial valid
THEN operasi login tetap sukses (tidak gagal karena audit)
AND `tracing::warn!` dipanggil untuk mencatat kegagalan audit
