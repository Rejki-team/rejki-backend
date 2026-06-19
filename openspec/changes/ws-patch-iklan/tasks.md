## 1. OpenSpec

- [x] 1.1 Buat proposal.md
- [x] 1.2 Buat design.md
- [x] 1.3 Buat specs/patch-iklan/spec.md
- [x] 1.4 Buat tasks.md

## 2. Barang Bekas

- [ ] 2.1 DTO — `UpdateBarangBekasInput` (semua field Option)
- [ ] 2.2 Repository trait — `update(id, seller_id, params) -> Result<Option<Entity>>`
- [ ] 2.3 PgRepository — SQL UPDATE COALESCE + warn_slow!
- [ ] 2.4 Service — `update()` with guard + rate limit + sanitasi
- [ ] 2.5 Handler — `update_iklan_barang_bekas`
- [ ] 2.6 Route — `PATCH /{id}` di protected router

## 3. Pekerja

- [ ] 3.1 DTO — `UpdatePekerjaInput` (semua field Option)
- [ ] 3.2 Repository trait — `update(id, poster_id, params) -> Result<Option<Entity>>`
- [ ] 3.3 PgRepository — SQL UPDATE COALESCE + warn_slow!
- [ ] 3.4 Service — `update()` with guard + rate limit + sanitasi
- [ ] 3.5 Handler — `update_iklan_pekerja`
- [ ] 3.6 Route — `PATCH /{id}` di protected router

## 4. Pekerjaan

- [ ] 4.1 DTO — `UpdatePekerjaanInput` (semua field Option)
- [ ] 4.2 Repository trait — `update(id, poster_id, params) -> Result<Option<Entity>>`
- [ ] 4.3 PgRepository — SQL UPDATE COALESCE + warn_slow!
- [ ] 4.4 Service — `update()` with guard + rate limit + sanitasi
- [ ] 4.5 Handler — `update_iklan_pekerjaan`
- [ ] 4.6 Route — `PATCH /{id}` di protected router

## 5. Pelatihan

- [ ] 5.1 DTO — `UpdatePelatihanInput` (semua field Option, ganti `UpdateIklanPelatihanInput` yang full-required)
- [ ] 5.2 Repository trait — `update(id, poster_id, params) -> Result<Option<Entity>>`
- [ ] 5.3 PgRepository — SQL UPDATE COALESCE + warn_slow!
- [ ] 5.4 Service — `update()` with guard (PelatihanStatus guard) + rate limit + sanitasi
- [ ] 5.5 Handler — `update_iklan_pelatihan`
- [ ] 5.6 Route — `PATCH /{id}` di protected router

## 6. Verifikasi

- [ ] 6.1 `cargo fmt --all`
- [ ] 6.2 `cargo clippy --workspace -- -D warnings`
- [ ] 6.3 `cargo check --workspace`
- [ ] 6.4 `cargo test --workspace`
- [ ] 6.5 `cargo sqlx prepare --workspace`

## 7. Dokumentasi

- [ ] 7.1 Update `docs/implementation-plan-phase-3.html`
- [ ] 7.2 Update `docs/index.html`
- [ ] 7.3 Update `rejki-app/src/openapi.rs`
- [ ] 7.4 Archive change
