## Why

Keempat entitas iklan (`pekerjaan`, `pekerja`, `barang-bekas`, `pelatihan`) saat ini menyimpan lokasi sebagai free-text (`lokasi TEXT`). Tidak ada referential integrity ke data wilayah — tidak bisa filter/group by region, tidak bisa validasi bahwa lokasi yang dimasukkan benar-benar ada. User-service sudah punya validasi KYC berbasis `RegionClient::get_region()`; sekarang saatnya menerapkan validasi yang sama ke semua iklan agar data wilayah konsisten di seluruh platform.

## What Changes

- **Tambah kolom `region_id TEXT`** (nullable) ke tabel `iklan` di schema `iklan_pekerjaan`, `iklan_pekerja`, `iklan_barang_bekas`, `iklan_pelatihan` — menyimpan `region.id` (kode wilayah) dari `region-service`
- **Tambah `region_id: Option<String>`** ke entity, DTO response, DTO input (create), dan repository params di keempat service
- **Integrasi `RegionClient`**: inject `Arc<dyn RegionClient>` ke service layer; pada create/update, jika `region_id` diisi, panggil `RegionClient::get_region(region_id)` — gagal dengan 422 jika region tidak ditemukan
- **Kolom `lokasi` tetap ada** — free-text tidak dihapus untuk backward compatibility; `region_id` adalah kolom tambahan terstruktur
- **Wire `region_client`** dari `rejki-app` ke keempat router iklan

## Capabilities

### New Capabilities

- `region-id-iklan`: Normalisasi referensi wilayah di 4 entitas iklan melalui `region_id TEXT` + validasi `RegionClient::get_region()` pada create dan update

### Modified Capabilities

*(tidak ada — ini adalah kemampuan baru, bukan perubahan requirement spesifikasi yang sudah ada)*

## Impact

- **4 iklan services**: entity, DTO, repository trait, pg_repository, application service, router
- **Migrations**: 4 file `up.sql` (1 per service) menambah kolom `region_id TEXT` + partial index
- **rejki-app**: wire `region_client` ke 4 router iklan (saat ini hanya user-service yang menerima `region_client`)
- **Dependency**: tambah `region-service-client` ke `Cargo.toml` keempat iklan service
- **Breaking**: Tidak ada — `region_id` bersifat opsional; `lokasi` tetap berfungsi seperti sebelumnya
