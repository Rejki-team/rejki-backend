# Design: ws-testing-coverage

> Workstream ini membangun foundation testing. Design doc fokus pada **keputusan tooling, tier
> test, fixture, dan strategi coverage** — bukan feature behavior.

## D1 — Pemilihan coverage tool: `cargo-llvm-cov` (bukan tarpaulin/nightly)

**Keputusan:** Pakai `cargo-llvm-cov` + `llvm-tools-preview`.

**Rasionale:**
- Task `fix-auth` 11.5 menandai coverage report sebagai "butuh nightly toolchain". Itu benar untuk
  versi lama `cargo-llvm-cov`, namun **modern `cargo-llvm-cov` berjalan di stable** + component
  `llvm-tools-preview`. Tidak perlu pin nightly.
- Lebih cepat & akurat dari `tarpaulin` (yang lambat di large workspace + sering false-negative
  di async code).
- Output LCOV → bisa di-upload ke Codecov/Coveralls atau diparse di CI untuk gate threshold.

**Implikasi CI:**
```yaml
- name: Install llvm-cov
  run: cargo install cargo-llvm-cov && rustup component add llvm-tools-preview
- name: Coverage
  working-directory: rust-services
  run: cargo llvm-cov --workspace --lcov --output-path lcov.info
```
Gate threshold via script parse summary (overall line coverage) → fail bila < 0.85.

**Catatan**: integration test butuh DB → coverage job juga butuh Postgres service container
(sama seperti integration-test job). Coverage tanpa integration test akan melaporkan
infrastructure layer rendah secara menyesatkan. Maka coverage job = integration + unit
dikombinasi.

## D2 — Tier test & penempatan (sesuai testing-standard.html)

| Tier | %target | Lokasi | DB | Mock |
|---|---|---|---|---|
| Unit — domain | ~100% | `*-service/src/**_test.rs` inline `#[cfg(test)]` | Tidak | Tidak perlu (pure logic) |
| Unit — application | ≥ 80% | `*-service/src/**_test.rs` + `Mock*Repository` di `tests/mock.rs` | Tidak | Ya (mock repo trait) |
| Integration — repository | ≥ 60% | `*-service/tests/pg_*_test.rs` `#[sqlx::test]` | Ya (`rejki_test`) | Tidak |
| Integration — interface/e2e | ≥ 70% | `rejki-app/tests/*_test.rs` (sudah ada, perluas) | Ya | Tidak |

**Penting (DIP dependency):** `fix-auth` task 9.x sudah extract `TokenIssuer`/`TokenValidator`
trait & `RateLimiter` trait (DIP). Ini memungkinkan `AuthService` diuji dengan mock tanpa `JwtService`
konkret → application layer testable tanpa RSA key. **Prasyarat ini sudah terpenuhi**, jadi unit
test application layer auth bisa langsung dibangun.

## D3 — Mock repository pattern (application layer testable tanpa DB)

**Keputusan:** Setiap service yang punya business logic non-trivial buat `Mock*Repository`
mengimplmenetasikan trait domain (mis. `MockAuthRepository: AuthRepository`). Ditaruh di
`*-service/tests/mock.rs` (hanya compile saat `#[cfg(test)]`).

**Rasionale:**
- CLAUDE.md §3: application layer generic atas trait repo (`Service<R: Repository>`), **bukan**
  `Box<dyn>`. Maka mock = concrete impl dari trait, di-inject generic. Tidak butuh `mockall` crate
  (tambah dependency) — mock manual sederhana dengan `Vec<...>` state cukup untuk assertion.
- `fix-auth` task 8.1 sudah membuat `MockAuthRepository` → lanjutkan pola yang sama ke user-service
  & service lain.

**Trade-off mock manual vs `mockall`:** mock manual = lebih verbose tapi zero dependency &
eksplisit. Konsisten dengan filosofi "minimal dependency" workspace. **Pilih mock manual.**

## D4 — Integration test: idempotensi & isolation

**Keputusan:** Integration test pakai `#[sqlx::test]` — otomatis transaction-per-test + rollback.
Untuk test yang butuh commit nyata (mis. cross-transaction behavior), pakai database terpisah
per test run atau TRUNCATE eksplisit di fixture setup.

**Rasionale:**
- `#[sqlx::test]` memberi transaction isolation gratis → idempoten, no cross-test pollution.
- Tapi beberapa test (mis. concurrent refresh token race — task `fix-auth` 2.4) butuh transaction
  terpisah yang commit. Solusi: spawn task tokio dengan koneksi pool terpisah, bukan
  `#[sqlx::test]` transaction wrapper. Pattern ini sudah ada di `rejki-app/tests/`.
- Fixture factory: email `@test.rejki.internal`, UUID unik per test → hindari collision bahkan
  tanpa rollback.

## D5 — CI schemas sync: 8 → 12

**Masalah:** `ci.yml` step "Init schemas" hanya buat 8 schema. Service sekarang: auth, user_svc,
chat, notification, iklan_pekerjaan, iklan_pekerja, iklan_barang_bekas, iklan_pelatihan, **region,
corporate_comms, report, storage** (4 baru).

**Keputusan:** Daripada hardcode daftar schema (rapuh — akan usang lagi saat service baru
ditambah), ubah step jadi **dinamis**: loop semua `*-service/migrations/` folder, derive schema
name, `CREATE SCHEMA IF NOT EXISTS`, lalu `sqlx migrate run`. Satu sumber kebenaran: folder
migrations.

**Implementasi sketsa:**
```bash
export DATABASE_URL="postgres://rejki_test:rejki_test@localhost:5432/rejki_test"
for dir in rust-services/*-service/migrations; do
  svc=$(basename $(dirname "$dir"))
  schema=$(echo "$svc" | sed 's/-/_/g')   # auth-service -> auth
  psql ... -c "CREATE SCHEMA IF NOT EXISTS $schema"
  sqlx migrate run --source "$dir" --database-url "$DATABASE_URL"
done
```

## D6 — Coverage gate: overall ≥ 85% sebagai gate, per-layer sebagai info

**Keputusan:** CI gate = **overall line coverage ≥ 85%** (Acceptance Criteria §2). Threshold
per-layer (application ≥ 80%, interface ≥ 70%, infrastructure ≥ 60%) dilaporkan sebagai **info/
warning**, bukan gate keras — karena overall ≥ 85% sudah cukup memaksa kualitas, dan memaksakan
per-layer gate bisa mendorong test "dummy" di layer yang sulit (mis. interface butuh banyak
HTTP fixture).

**Rasionale:** Acceptance Criteria §2 eksplisit: "overall coverage ≥ 85% adalah Acceptance
Criteria" — per-layer adalah panduan internal testing-standard, bukan kriteria hard gate.

**Fail behavior:** Job coverage `exit 1` bila overall < 85%. Comment PR dengan ringkasan
per-layer untuk visibilitas.

## D7 — Urutan eksekusi (anti-bottleneck)

1. **CI fix dulu** (schema sync + jalankan semua integration test) — ini paling cepat & paling
   tinggi value: membuka jalan untuk mengetahui test mana yang sudah hijau/merah.
2. **Unit test auth-service** (paling banyak business logic, paling banyak deferred task).
3. **Unit test user-service** (tutup 11 task deferred).
4. **Integration test gap** (concurrent refresh, partial-failure NIK, test file yang belum
   dijalankan).
5. **Coverage gate CI** (setelah test cukup, pasang gate).
6. **Roll-out ke service lain** (chat, iklan-*, corporate-comms, report, region, storage) —
   bisa bertahap, tidak block P3 mulai selama auth+user+gate sudah ada.

Urutan ini memastikan: coverage gate dipasang **setelah** ada test yang cukup (bukan gate kosong
yang langsung fail), dan P3 (security) bisa mulai begitu auth+user teruji.

## D8 — Risiko & mitigasi

| Risiko | Mitigasi |
|---|---|
| Durasi CI naik signifikan (coverage instrumentation + semua test) | `rust-cache`, parallel job, `cargo-nextest` untuk paralelisasi test cepat |
| Flaky integration test (DB timing, port) | `--test-threads=1`, fixture idempoten, retry bursa bila perlu |
| Coverage < 85% saat gate pertama dipasang (utang lama) | Gate dipasang **bertahap**: threshold mulai dari % aktual saat ini, naik bertahap ke 85% dalam beberapa PR — **atau** pasang gate sebagai "report dulu, fail kemudian" (lihat D9) |
| Mock manual verbose → god-test | Pecah per use case; satu mock file per service |

## D9 — Strategi rollout gate (soft → hard)

Karena coverage saat ini mungkin jauh di bawah 85% (0 unit test di crate service), pasang gate
hard 85% langsung = CI langsung merah. **Dua opsi:**

- **Opsi A (rekomendasi):** Gate = `cargo-llvm-cov` report + parse overall. Threshold awal =
  %aktual (mis. 40%) saat P2 mulai, lalu naikkan threshold per PR sampai 85%. Gate hard 85%
  dicapai sebelum P2 di-archive.
- **Opsi B:** Gate langsung 85% — hanya feasible bila P2 sekaligus menulis semua test sampai
  coverage 85%. Lebih agresif, risiko CI merah lama.

**Pilih Opsi A** — lebih realistis & tidak memblokir kolaborasi (PR lain tidak diblokir selama
P2 berjalan). CLAUDE.md §2 mengizinkan: "Jika kriteria tidak bisa dipenuhi 100% pada satu
iterasi, catat eksplisit statusnya" — gate bertahap adalah bentuk pencatatan eksplisit itu.
