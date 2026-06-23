## 1. Migrations (DB columns)

- [x] 1.1 Tambah migration `region_id` di `iklan-pekerjaan-service/migrations/`
- [x] 1.2 Tambah migration `region_id` di `iklan-pekerja-service/migrations/`
- [x] 1.3 Tambah migration `region_id` di `iklan-barang-bekas-service/migrations/`
- [x] 1.4 Tambah migration `region_id` di `iklan-pelatihan-service/migrations/`

## 2. Dependency: region-service-client

- [x] 2.1 Tambah `region-service-client = { path = "../region-service-client" }` ke `iklan-pekerjaan-service/Cargo.toml`
- [x] 2.2 Tambah `region-service-client = { path = "../region-service-client" }` ke `iklan-pekerja-service/Cargo.toml`
- [x] 2.3 Tambah `region-service-client = { path = "../region-service-client" }` ke `iklan-barang-bekas-service/Cargo.toml`
- [x] 2.4 Tambah `region-service-client = { path = "../region-service-client" }` ke `iklan-pelatihan-service/Cargo.toml`

## 3. Domain layer: Entity + Repository trait

- [x] 3.1 Tambah `region_id: Option<String>` ke `IklanPekerjaan` entity + `CreatePekerjaanParams` + `to_response`/`to_admin_response`
- [x] 3.2 Tambah `region_id: Option<String>` ke `IklanPekerja` entity + `CreatePekerjaParams` + response/DTO
- [x] 3.3 Tambah `region_id: Option<String>` ke `IklanBarangBekas` entity + `CreateBarangBekasParams` + response/DTO
- [x] 3.4 Tambah `region_id: Option<String>` ke `IklanPelatihan` entity + `CreatePelatihanParams` + `UpdatePelatihanParams` + response/DTO

## 4. Infrastructure layer: pg_repository

- [x] 4.1 Update semua query SQL + `row_to_entity` di `PgIklanPekerjaanRepository` (tambah kolom `region_id`)
- [x] 4.2 Update semua query SQL + `row_to_entity` di `PgIklanPekerjaRepository` (tambah kolom `region_id`)
- [x] 4.3 Update semua query SQL + `row_to_entity` di `PgIklanBarangBekasRepository` (tambah kolom `region_id`)
- [x] 4.4 Update semua query SQL + `row_to_entity` di `PgIklanPelatihanRepository` (tambah kolom `region_id`)

## 5. Application layer: Service + validation

- [x] 5.1 Inject `RegionClient` + validasi di `IklanPekerjaanService::create()`
- [x] 5.2 Inject `RegionClient` + validasi di `IklanPekerjaService::create()`
- [x] 5.3 Inject `RegionClient` + validasi di `IklanBarangBekasService::create()`
- [x] 5.4 Inject `RegionClient` + validasi di `IklanPelatihanService::create()` dan `update_pelatihan()`

## 6. Interface layer: Router wiring

- [x] 6.1 Update `iklan_pekerjaan_service::router()` — terima `Option<Arc<dyn RegionClient>>`
- [x] 6.2 Update `iklan_pekerja_service::router()` — terima `Option<Arc<dyn RegionClient>>`
- [x] 6.3 Update `iklan_barang_bekas_service::router()` — terima `Option<Arc<dyn RegionClient>>`
- [x] 6.4 Update `iklan_pelatihan_service::router()` — terima `Option<Arc<dyn RegionClient>>`

## 7. Composition Root (rejki-app)

- [x] 7.1 Wire `region_client.clone()` ke keempat `router()` iklan di `rejki-app/src/main.rs`

## 8. Verify

- [x] 8.1 `cargo check --workspace` — kompilasi sukses
- [x] 8.2 `cargo clippy --workspace -- -D warnings` — zero warning
- [x] 8.3 `cargo fmt --all` — format
- [x] 8.4 `cargo test --workspace` — semua test lulus
- [x] 8.5 `cargo sqlx prepare --workspace` — update offline cache (bila DB tersedia)
