# CLAUDE.md — rejki-backend

Panduan kerja untuk Claude Code di repo `rejki-backend`. Dokumen ini WAJIB diikuti pada
setiap tugas. Tujuannya: setiap perubahan selaras dengan dokumentasi (`docs/`), spesifikasi
(`openspec/`), aturan **Phase 1** & **Phase 1.5**, dan **Acceptance Criteria** di bawah.

> Bahasa kerja & komunikasi: **Bahasa Indonesia** (samakan dengan dokumentasi & komentar repo).

---

## 0. Aturan Emas (baca dulu sebelum apa pun)

1. **Jangan halu, jangan berasumsi tanpa dasar.** Jika tidak yakin → **tanya** atau lakukan
   **Web Search** untuk menambah keyakinan. Verifikasi nama file/symbol/flag sebelum dipakai.
2. **Validasi tiga sumber kebenaran** untuk setiap alur sebelum koding:
   - Dokumentasi terkait di [`docs/`](docs/)
   - Spesifikasi di [`openspec/`](openspec/) (proposal + spec + tasks + design)
   - **Acceptance Criteria** (§2 di bawah)
3. **Pekerjaan harus sesuai konteks domain-nya.** Jangan mencampur logika lintas domain.
4. **Hormati boundary arsitektur** (§3) — pelanggaran layer/dependency = blocker, bukan nit.
5. Setelah implementasi temuan, **buat/Update Swagger** (khusus environment `development`)
   dan **buat dokumentasinya**.
6. Pastikan **semua task pada OpenSpec terimplementasi sempurna** sebelum dianggap selesai.

---

## 1. Peta Repo & Stack

`rejki-backend` adalah **modular monolith** (Cargo workspace) yang siap diekstrak menjadi
microservices. Ada **dua stack bahasa**:

| Area | Lokasi | Stack | Aturan yang berlaku |
|---|---|---|---|
| **Backend utama** | [`rust-services/`](rust-services/) | **Rust + Axum** | §3, §4, §5 (**RUST+AXUM**) |
| **Notification consumer** | [`bun-notification-service/`](bun-notification-service/) | **Bun.js + TypeScript** | §6 (**LAIN**) |
| Infra & deploy | root, [`.github/`](.github/), `nginx.*.conf`, `docker-compose.yml` | YAML/Nginx/Podman | §7 |
| Dokumentasi | [`docs/`](docs/) | HTML/MD/PlantUML | §0, §8 |
| Spesifikasi | [`openspec/`](openspec/) | Markdown (spec-driven) | §8 |

> **Penting:** Aturan di §4 dan §5 (Clean Architecture, SOLID, error/retry/IO, `warn_slow!`,
> trait native async, `Box<dyn>` vs generic, dsb.) adalah **khusus Rust + Axum**. Untuk
> Bun.js/TypeScript atau framework lain, gunakan **§6** — jangan paksakan idiom Rust ke sana.

### Entry Point (main) tiap aplikasi

| Aplikasi | Stack | Entry point (main) | Swagger / OpenAPI |
|---|---|---|---|
| **rejki-app** (backend utama) | Rust + Axum | [`rust-services/rejki-app/src/main.rs`](rust-services/rejki-app/src/main.rs) (`#[tokio::main]`, Composition Root) | **Ya** — di-wire dari `main.rs` (`mod openapi;`) lewat [`rust-services/rejki-app/src/openapi.rs`](rust-services/rejki-app/src/openapi.rs), **gated `APP_ENV=development`** |
| **bun-notification-service** | Bun.js + TS | [`bun-notification-service/src/index.ts`](bun-notification-service/src/index.ts) (consumer loop + SIGTERM/SIGINT) | **Tidak ada** — service ini tanpa HTTP server (Redis Streams consumer), jadi tidak punya Swagger |

> **Update Swagger → di main app, yaitu `rejki-app`.** Karena seluruh route service di-`nest`
> di bawah `rejki-app` (prefix `/api/v1/…`), dokumentasi OpenAPI **tersentralisasi di
> `rejki-app/src/openapi.rs`**, bukan tersebar di tiap `*-service`. Pola ini sengaja: per
> design **D7 (composition root mirror)** — DTO doc adalah "mirror" lokal di `rejki-app`,
> sehingga crate domain (`auth-service`, dst.) **tidak** menarik dependency `utoipa` hanya demi
> Swagger. Saat menambah/mengubah endpoint, perbarui mirror DTO + path di file itu.

### Struktur `rust-services/` (workspace)

- `rejki-app/` — **Composition Root** (binary). Hanya wiring + middleware global. **HANYA boleh
  meng-import `*-service` (impl), TIDAK boleh `*-service-client`.** Tidak ada domain client di sini.
- `*-service/` (mis. `auth-service`) — implementasi penuh satu domain. Dual target: `[lib]` +
  `[[bin]]`. 4 layer: `domain/ application/ infrastructure/ interface/`.
- `*-service-client/` (mis. `auth-service-client`) — **kontrak publik** (trait + DTO + Error
  enum). Tanpa axum/sqlx/tokio. **Komunikasi antar-service WAJIB lewat client domain ini.**
- `common/` — `crypto`, `config`, `errors`, `tracing-setup`, `auth-middleware`.

Service saat ini: `region`, `storage`, `auth`, `user`, `chat`, `notification`,
`iklan-pekerjaan`, `iklan-pekerja`, `iklan-barang-bekas`, `iklan-pelatihan`,
`corporate-comms`, `report` (lihat [`rust-services/Cargo.toml`](rust-services/Cargo.toml)).

---

## 2. Acceptance Criteria (WAJIB untuk SEMUA tugas)

Setiap perubahan diukur terhadap kriteria ini. Tandai mana yang stack-spesifik.

**Arsitektur & kualitas (semua stack):**
- Menerapkan **Clean Architecture**.
- Menerapkan **SOLID Principle**.
- Menerapkan **Unit Testing** minimal **overall coverage ≥ 85%**.
- Menerapkan **struktur data yang optimal**.
- Menerapkan **script query yang optimal & tidak lambat**.
- Menerapkan **pengelolaan I/O yang aman**.
- Menerapkan **retry yang aman** sesuai best practice saat ini.
- Menerapkan **error handling yang aman** sesuai best practice saat ini.
- Menerapkan aturan & standarisasi **Phase 1 & Phase 1.5**.
- Menerapkan **code formatting** sesuai aturan bahasa yang dipakai.
- **Tidak** memakai library/crate/sintaksis yang **deprecated** atau masih **eksperimental**.

**Konkurensi & memori (terutama RUST+AXUM):**
- **Memory safe** & **Thread safe**.
- **Zero Race Condition**, **Zero Connection Leak**.
- **Zero Too Many Arguments** (Rust) — refactor jadi struct param bila argumen banyak.

**Domain & komposisi:**
- `rejki-app` **hanya full implementasi service**, **tidak boleh ada domain client**.
- **Komunikasi antar service memakai domain client-nya.**
- **Pekerjaan harus sesuai konteks domain-nya.**

**Konfigurasi & rahasia:**
- Semua konfigurasi/setup VPS & aplikasi memakai **GitHub Secret & Variable**, kecuali
  beberapa yang memang hanya bisa lewat `.env`.
- **Zero Hardcoded** (URL, kredensial, magic number tanpa konstanta bernama).

**Anti-pattern yang harus NOL:**
- **Zero N+1 Query**, **Zero God Function**, **Zero God Class**, **Zero Security Issue**,
  **Zero Circular Dependency**.

**Best practice:**
- Menerapkan **Best Practice pembuatan backend Rust + Axum saat ini**.

> Jika sebuah kriteria tidak bisa dipenuhi 100% pada satu iterasi (mis. coverage 85% perlu
> refactor DIP besar), **catat eksplisit** statusnya di tasks OpenSpec + dokumentasi —
> jangan diam-diam dilewati.

---

## 3. Boundary Arsitektur (RUST+AXUM) — non-negotiable

Empat layer per service, arah dependency satu arah:

```
interface  → application → domain ← infrastructure
  (Axum)      (use case)   (entity,     (sqlx impl
   DTO         orchestrate  trait repo)   trait repo)
```

- `domain/` — entity, value object, **trait repository**. Tidak tahu DB maupun HTTP.
- `application/` — use case. Hanya boleh import `domain`. Generic atas trait repo
  (`Service<R: Repository>`), **bukan** `Box<dyn Trait>` (hindari vtable overhead, wiring
  terverifikasi compile-time).
- `infrastructure/` — implementasi konkret trait domain (PostgreSQL via `sqlx`), `JwtService`,
  Redis publisher, dsb. **JWT/Redis/S3 adalah detail infrastructure, bukan domain.**
- `interface/` — handler Axum, DTO request/response, `pub fn router(pool) -> Router`.

Aturan turunan:
- `domain` **tidak boleh** import `infrastructure` atau `interface`.
- Wiring konkret hanya di `router()` atau di `rejki-app`.
- **Composition Root** (`rejki-app`) import `*-service` (impl), bukan `*-service-client`.
- **Antar service** → lewat `*-service-client` trait (mis. `AuthClient`,
  `NotificationClient`). Saat ekstraksi, impl in-process ditukar HTTP tanpa ubah pemanggil.
- `common/auth-middleware` **tidak decode JWT sendiri** — delegasi ke
  `Arc<dyn AuthClient>` (Dependency Inversion). Private key RSA **hanya** di `auth-service`.

---

## 4. Aturan Rust + Axum (Phase 1 & 1.5) — **RUST+AXUM**

Semua di bagian ini **khusus** untuk `rust-services/`.

### 4.1 Dependency & versi
- **Semua versi dipinned di workspace root** `rust-services/Cargo.toml`; tiap crate pakai
  `{ workspace = true }`. Jangan deklarasi versi berbeda per-crate.
- Native `async fn in trait` (Rust ≥ 1.75) — **jangan tambah `async-trait`** kecuali memang
  dibutuhkan untuk `dyn` trait object (mis. `dyn AuthClient`); itu satu-satunya pengecualian.
- `*-service-client` **tanpa** axum/sqlx/tokio — hanya trait + DTO + Error enum.

### 4.2 Error handling & response
- `common/errors::AppError` → `IntoResponse` JSON body `{ "error": <CODE>, "message": <str> }`.
  Variant: `NotFound, Unauthorized, Validation, Conflict, Forbidden, Internal, Gone`.
- Sukses dibungkus `ApiResponse<T>` (field `success, data, meta, request_id` UUID v7).
- POST create → **201 + header `Location: /api/v1/{resource}/{id}`** via `created_response()`.
  DELETE → **204**. Validasi gagal → **422** lewat extractor `ValidatedJson<T>` (reject
  sebelum masuk handler).
- **Stack trace / detail internal tidak pernah ke client.** `AppError::Internal` → log `error!`,
  client hanya dapat pesan generik.

### 4.3 Database & query (lihat `docs/database-convention.html`)
- Satu **schema PostgreSQL per service**. **Tidak ada cross-schema FK / JOIN** — antar service
  via API/client.
- Kolom wajib tiap tabel: `id UUID PK DEFAULT gen_random_uuid()`, `created_at`, `updated_at`
  `TIMESTAMPTZ NOT NULL DEFAULT now()`. Soft-delete → `deleted_at TIMESTAMPTZ` (NULL = aktif).
- Semua nama `snake_case`. Index: `idx_{tabel}_{kolom}`, unique `uq_…`, FK constraint `fk_…`.
  **Wajib index untuk tiap kolom FK-equivalent** dan kolom filter (`email`, `status`, …).
  Pakai **partial/composite index** untuk hindari scan → **Zero N+1 & query lambat**.
- `updated_at` di-update **eksplisit di query** (`SET updated_at = now()`), **bukan trigger**.
- Query soft-delete **wajib** `AND deleted_at IS NULL`.
- **100% parameterized query via `sqlx`** — tidak ada string concatenation SQL.
- Update offline cache bila ada query baru: `cargo sqlx prepare --workspace` (lihat
  `rust-services/.sqlx/`).
- Migration: satu perubahan per file, **tidak destruktif**, **tidak diedit** setelah apply,
  hanya DDL (tanpa data mutation), kolom baru `NULLABLE` dulu.

### 4.4 Autorisasi (lihat `docs/authorization-pattern.html`)
- 3 lapis: **autentikasi di middleware**, **ownership check di repository**, **visibility di handler**.
- **IDOR → 404, bukan 403.** Ownership check **di query** (`WHERE id = $1 AND owner_id = $2
  AND deleted_at IS NULL`), bukan `SELECT` lalu compare (race + bocor 403).
- `403` **hanya** untuk pembatasan fitur (mis. tier premium), bukan IDOR.

### 4.5 Konkurensi, memori, I/O, retry
- **Memory & thread safe**: share state via `Arc<…>`; **jangan** lock di hot path tanpa alasan.
- **Zero Race Condition**: operasi gabungan harus **atomik** — bungkus dalam **transaction**
  (`pool.begin()` … `commit()`), atau pola DELETE-RETURNING (mis. consume OTP, rotasi refresh
  token). Lihat `fix-auth-service-code-review-findings` sebagai contoh kanonik.
- **Zero Connection Leak**: pakai pool `sqlx`; jangan tahan koneksi/transaction lewat `.await`
  yang panjang; pastikan setiap `begin()` berakhir `commit`/`rollback`.
- **I/O aman**: timeout eksplisit; degradasi anggun (mis. Redis down → DB tetap simpan, push
  di-skip dengan `warn!`). Validasi & sanitasi semua input I/O eksternal.
- **Retry aman**: idempoten, **exponential backoff + jitter**, batas attempt, dan **DLQ** untuk
  yang gagal permanen (pola consumer notification). Jangan retry operasi non-idempoten tanpa
  guard idempotency.
- Rate limiter Redis: **selalu `EXPIRE` setelah `INCR`** (cegah permanent lockout); **fail-open**
  + `warn!` bila Redis tak ada.

### 4.6 Observability
- `warn_slow!($start, $op)` di tiap method repository — `WARN` jika query > 100ms
  (field `op`, `elapsed_ms`).
- `request_id_layer` (UUID v7) inject + echo `x-request-id`. Log JSON di `production`, pretty di
  `development` (`common/tracing-setup`).

### 4.7 Anti God-Function / God-Class / Too-Many-Args
- Fungsi fokus satu tanggung jawab; pecah use case besar jadi helper privat.
- **Argumen banyak → bungkus jadi struct input** (mis. `RegisterInput`), bukan 5+ argumen lepas.
- Hindari `clippy::too_many_arguments`; jangan `#[allow(...)]` kecuali ada justifikasi tertulis.

### 4.8 Testing (lihat `docs/testing-standard.html`)
- Naming: `test_{unit}_given_{kondisi}_when_{aksi}_then_{ekspektasi}`.
- Unit test murni inline `#[cfg(test)]`; integration test di `tests/` pakai **DB nyata**
  (`#[sqlx::test]`), idempoten, fixture factory (email `@test.rejki.internal`), **bukan** mock DB.
- Wajib ada test IDOR `…_given_other_user_…_then_returns_404`.
- **Target coverage overall ≥ 85%** (CI: `cargo llvm-cov`/`tarpaulin`). Layer application ≥ 80%,
  interface ≥ 70%, infrastructure ≥ 60% — tapi overall **≥ 85%** adalah Acceptance Criteria.

### 4.9 Perintah wajib sebelum "selesai" (RUST+AXUM)
Jalankan dari `rust-services/` (manifest: `rust-services/Cargo.toml`):
```bash
cargo fmt --all                       # format (WAJIB, code formatting Rust)
cargo clippy --workspace -- -D warnings   # zero warning
cargo check --workspace               # kompilasi lintas service
cargo test --workspace --lib          # unit test (tanpa integration test)
```
> Integration test tidak dijalankan di CI — dilakukan manual saat development dan staging.
> Untuk menjalankan integration test lokal, butuh PostgreSQL + RSA key di `./keys/`.

---

## 5. Best Practice Axum saat ini (RUST+AXUM)

- `axum = 0.8`, `tokio` full, handler async; state via `State<T>` / `Extension<T>`.
- Middleware global di `rejki-app`: `request_id_layer` → `CompressionLayer` → `TraceLayer` →
  `CorsLayer`. Semua route di bawah prefix `/api/v1/`. `GET /health` tanpa auth.
- **Graceful shutdown**: `ctrl_c()` + `SIGTERM` (unix), drain.
- **Fail-fast config** di startup (`common/config::AppConfig::from_env`): panic dengan pesan
  jelas bila var wajib hilang/invalid. `.env` di-load **hanya** saat `APP_ENV=development`.
- Swagger via `utoipa` + `utoipa-swagger-ui` **di-gate hanya `APP_ENV=development`** (lihat
  `rejki-app/src/openapi.rs`).
- WebSocket: validasi JWT **sebelum upgrade** (401 bila invalid), `WsEnvelope`, close codes
  (4001/4003/4004/1012), keepalive 60s.

---

## 6. Aturan untuk Bahasa/Framework LAIN — **LAIN (non-Rust)**

Bagian §3, §4, §5 **TIDAK** dipaksakan ke sini. Untuk komponen non-Rust:

### 6.1 Bun.js / TypeScript — `bun-notification-service/`
Stack: **Bun + TypeScript**, deps `firebase-admin`, `ioredis`, `nodemailer`, `pg`.

- **Code formatting & lint** sesuai ekosistem TS (Prettier/ESLint atau formatter Bun), **bukan**
  `cargo fmt`. TypeScript `strict` mode; **tanpa `any`** kecuali berjustifikasi.
- **Clean Architecture / SOLID tetap berlaku secara konseptual**, tapi idiomnya TypeScript:
  pisahkan `config` / `db` / `firebase` / `email` / `consumer` (sudah tercermin di `src/`).
  Tidak ada "trait generic Rust" — gunakan interface + dependency injection TS.
- **Error handling**: jangan bocorkan detail internal; log terstruktur. **Retry** consumer:
  `XREADGROUP` + `XAUTOCLAIM` (recovery PEL), **idempotency** via
  `INSERT … ON CONFLICT DO NOTHING`, **max retry 3×** lalu `XADD …_dlq` + `XACK`.
- **I/O aman & graceful shutdown**: handle `SIGTERM`/`SIGINT`, tutup koneksi Redis/PG bersih.
- **Konfigurasi fail-fast**: helper `require_env()` (panic bila var wajib hilang). Nama env
  `SCREAMING_SNAKE_CASE`, konsisten dengan katalog Config Standard. **Zero hardcoded**.
- **DB convention** PostgreSQL tetap sama (§4.3): `snake_case`, kolom mandatory, tanpa
  cross-schema FK, migrasi DDL-only.
- **Testing**: pakai test runner Bun (`bun test`); tetap kejar perilaku kritis (idempotency,
  DLQ, parsing event). Naming test deskriptif.
- Perintah: `bun install --frozen-lockfile`, `bun run dev`, `bun test`. **Bukan** perintah cargo.

### 6.2 Framework/bahasa lain yang muncul kemudian (web/mobile/infra)
Untuk `rejki-web`, `rejki-mobile`, atau stack baru lain bila pekerjaan menyentuhnya:
- Ikuti **formatter & linter native** bahasa/framework tsb (mis. Prettier+ESLint untuk web,
  `dart format` untuk Flutter, `hadolint`/`yamllint` untuk infra).
- Pertahankan **prinsip lintas-stack**: Clean Architecture & SOLID secara konseptual, error
  handling aman, retry idempoten + backoff, I/O aman, **Zero Hardcoded / Zero Security Issue**,
  konfigurasi via GitHub Secret/Variable.
- **Jangan** menerapkan aturan Rust-spesifik (lifetime, `Box<dyn>`, `cargo` commands,
  `warn_slow!`, `too_many_arguments`) ke kode non-Rust.
- Komunikasi ke backend **lewat kontrak API** yang sama (`/api/v1/…`, `ApiResponse` envelope).

---

## 7. Infrastruktur, Secret, Podman, Swagger

- **Container runtime: Podman CLI** (gunakan `podman machine` bila perlu). Mapping mental:
  `docker compose` → `podman compose` / `podman-compose`. File compose: `docker-compose.yml`
  (postgres, redis, rejki-app, bun, nginx, cloudflare tunnel).
- **Secret/Variable**: produksi & CI/CD memakai **GitHub Secrets & Variables**
  (`VPS_HOST`, `VPS_USER`, `VPS_SSH_KEY`, kredensial DB/JWT/FCM/R2, dst). **Tidak ada `.env` di
  server.** `.env` **hanya** untuk development lokal; `.env.example` (placeholder, **tanpa nilai
  nyata**) wajib di-commit & selalu sinkron. Pengecualian `.env` hanya bila memang tak ada jalur
  Secret yang memungkinkan — dan catat alasannya.
- **Jangan commit** `keys/`, `*.pem`, `.env`, `.env.test` (pastikan di `.gitignore`).
- **Swagger**: setelah implementasi temuan selesai, **buat/Update Swagger di main app
  (`rejki-app`)** — edit [`rust-services/rejki-app/src/openapi.rs`](rust-services/rejki-app/src/openapi.rs)
  (mirror DTO + path), **hanya** untuk environment `development` (di-gate `APP_ENV=development`
  di `main.rs`). `bun-notification-service` tidak punya Swagger (tanpa HTTP server).
- CI (`.github/workflows/ci.yml`): `fmt` → `clippy` → `build` → `unit test (lib-only)` → `bun check` → `docker build check` → `coverage (lib-only)`.

---

## 8. Alur Kerja Wajib (per tugas)

1. **Pahami konteks**: baca proposal/spec/tasks/design terkait di `openspec/changes/<nama>/`,
   plus dokumen `docs/` yang relevan. Konfirmasi domain yang disentuh.
2. **Validasi alur** terhadap tiga sumber (§0.2) + Acceptance Criteria (§2). Bila ada
   ketidaksesuaian/ambiguitas → **tanya** atau Web Search; jangan asumsi.
3. **Tentukan stack** (Rust+Axum → §3–5; non-Rust → §6) dan terapkan aturan yang sesuai saja.
4. **Implementasi** sesuai boundary & best practice. Hormati "Zero …" di §2.
5. **Uji**: unit + integration; kejar overall coverage ≥ 85%.
6. **Format & lint** sesuai bahasa (cargo fmt/clippy untuk Rust; Prettier/ESLint untuk TS).
7. **Update Swagger** (dev) + **buat/Update dokumentasi** (`docs/` dan/atau `openspec`).
8. **Tandai semua task OpenSpec** yang relevan selesai; bila ada yang ditunda, catat alasannya.
9. **Laporkan apa adanya** (test yang gagal, langkah yang di-skip, tooling yang tak tersedia).

### OpenSpec
Perubahan dikelola spec-driven (lihat `.claude/commands/opsx/`, `.github/prompts/opsx-*`).
Sebelum koding fitur baru: ada **proposal** + **spec** + **tasks**. Setelah selesai &
ter-deploy: **archive** perubahannya. Jangan menandai task selesai sebelum benar-benar
terimplementasi & terverifikasi.

### Git Flow — Aturan Ketat (WAJIB diikuti)

**Prinsip: Setiap pekerjaan = 1 branch → 1 PR → merge ke `develop`.**

Langkah-langkah WAJIB yang harus dilakukan agent untuk setiap tugas:

1. **Sync & Start:**
   ```bash
   git checkout develop
   git pull origin develop
   ```

2. **Buat branch baru** dari `develop`:
   ```bash
   git checkout -b feat/ws-<nama-fitur>
   ```
   - Nama branch: `feat/ws-<nama-fitur>` (kebab-case)
   - Contoh: `feat/ws-deployment-observability`, `feat/ws-audit-log`

3. **Kerjakan tugas** → **Commit setelah 1 point pekerjaan selesai** (bukan bertahap per file):
   ```bash
   git add -A
   git commit -m "type: deskripsi singkat"
   ```

4. **Push + PR ke `develop`**:
   ```bash
   git push origin feat/ws-<nama-fitur>
   ```
   Kemudian buka GitHub → PR dari branch → `develop`.

5. **Setelah PR di-merge**, hapus branch lokal:
   ```bash
   git branch -d feat/ws-<nama-fitur>
   git checkout develop && git pull origin develop
   ```

**Aturan tambahan:**
- **Tidak menggabungkan multiple task dalam satu branch.** Jika terlanjur, cherry-pick commit ke branch masing-masing.
- **Jangan merge sendiri** — selalu lewat PR.
- **Branch dihapus** setelah PR di-merge ke `develop`.

---

## 9. Referensi Cepat

| Topik | File |
|---|---|
| Foundation, layer, JWT, wiring | `docs/phase-1-setup.html` |
| 9 wave Phase 1.5 (status & scope) | `docs/implementation-plan-phase-1.5.html` |
| Envelope API, versioning, 201/204/410 | `docs/api-standard.html`, `docs/api-versioning.html` |
| IDOR 404, ownership, visibility | `docs/authorization-pattern.html` |
| Schema, kolom mandatory, index, migrasi | `docs/database-convention.html` |
| Naming env, fail-fast, katalog var | `docs/config-standard.html` |
| `request_id`, `warn_slow!`, log JSON | `docs/logging-standard.html` |
| Naming test, tiers, coverage | `docs/testing-standard.html` |
| JWT, bcrypt, input sanitization, secret | `docs/security-baseline.html` |
| WebSocket envelope & close codes | `docs/websocket-contract.html` |
| Kontrak notifikasi (Redis Streams/FCM) | `docs/notification-contract.html` |
| Code Review (connection leaks, memory, N+1, security) | `docs/code-review-standards.html` |
| Workspace & versi crate | `rust-services/Cargo.toml` |
| Composition Root & wiring | `rust-services/rejki-app/src/main.rs` |
| Contoh service rujukan (4 layer + atomik) | `rust-services/auth-service/src/` |
| Bun consumer (retry/DLQ/idempotency) | `bun-notification-service/src/` |

> Dokumen `docs/*.html` di-render dari standar Phase 1.5. Bila isinya bertentangan dengan
> kode terbaru, **konfirmasi ke user** sebelum mengikuti salah satunya.
