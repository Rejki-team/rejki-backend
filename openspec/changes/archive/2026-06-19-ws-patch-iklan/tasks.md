## 1. OpenSpec

- [x] 1.1 Buat proposal.md
- [x] 1.2 Buat design.md
- [x] 1.3 Buat specs/patch-iklan/spec.md
- [x] 1.4 Buat tasks.md

## 2. Barang Bekas

- [x] 2.1 DTO — `UpdateBarangBekasInput` (semua field Option)
- [x] 2.2 Repository trait — `update(id, seller_id, params) -> Result<Option<Entity>>`
- [x] 2.3 PgRepository — SQL UPDATE COALESCE + warn_slow!
- [x] 2.4 Service — `update()` with guard + rate limit + sanitasi
- [x] 2.5 Handler — `update_iklan_barang_bekas`
- [x] 2.6 Route — `PATCH /{id}` di protected router

## 3. Pekerja

- [x] 3.1 DTO — `UpdatePekerjaInput` (semua field Option)
- [x] 3.2 Repository trait — `update(id, poster_id, params) -> Result<Option<Entity>>`
- [x] 3.3 PgRepository — SQL UPDATE COALESCE + warn_slow!
- [x] 3.4 Service — `update()` with guard + rate limit + sanitasi
- [x] 3.5 Handler — `update_iklan_pekerja`
- [x] 3.6 Route — `PATCH /{id}` di protected router

## 4. Pekerjaan

- [x] 4.1 DTO — `UpdatePekerjaanInput` (semua field Option)
- [x] 4.2 Repository trait — `update(id, poster_id, params) -> Result<Option<Entity>>`
- [x] 4.3 PgRepository — SQL UPDATE COALESCE + warn_slow!
- [x] 4.4 Service — `update()` with guard + rate limit + sanitasi
- [x] 4.5 Handler — `update_iklan_pekerjaan`
- [x] 4.6 Route — `PATCH /{id}` di protected router

## 5. Pelatihan

- [x] 5.1 DTO — `UpdatePelatihanInput` (semua field Option, ganti `UpdateIklanPelatihanInput` yang full-required)
- [x] 5.2 Repository trait — `update(id, poster_id, params) -> Result<Option<Entity>>`
- [x] 5.3 PgRepository — SQL UPDATE COALESCE + warn_slow!
- [x] 5.4 Service — `update()` with guard (PelatihanStatus guard) + rate limit + sanitasi
- [x] 5.5 Handler — `update_iklan_pelatihan`
- [x] 5.6 Route — `PATCH /{id}` di protected router

## 6. Verifikasi

- [x] 6.1 `cargo fmt --all`
- [x] 6.2 `cargo clippy --workspace -- -D warnings`
- [x] 6.3 `cargo check --workspace`
- [x] 6.4 `cargo test --workspace`
- [x] 6.5 `cargo sqlx prepare --workspace`

## 7. Dokumentasi

- [x] 7.1 Update `docs/implementation-plan-phase-3.html`
- [x] 7.2 Update `docs/index.html`
- [x] 7.3 Update `rejki-app/src/openapi.rs`
- [x] 7.4 Archive change
