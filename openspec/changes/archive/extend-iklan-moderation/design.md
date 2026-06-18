## Context

Keempat service iklan berbagi pola identik (penelusuran kode 2026-06-14): entity dengan flag boolean (`is_active` untuk pekerjaan/pekerja/pelatihan, `is_sold` untuk barang), owner `poster_id` (kecuali `seller_id` di barang), route `GET /`, `GET /{id}`, `POST /`, `DELETE /{id}` (hard delete), query `limit/offset` saja. Tidak ada state moderasi, soft-delete, search, CSV, atau endpoint admin. Beberapa field (`lokasi`, `gaji_*`/`tarif_*`/`harga`, `tanggal_*`, `foto_urls`) ada di entity tetapi **tidak ikut di-INSERT/di-surface**. `StorageClient` sudah mendukung presigned upload berkategori (`avatar`/`ktp`/`selfie`/`suspension-evidence`) dengan verifikasi mime/magic-bytes; `NotificationClient` mendukung `send`/`send_bulk`/`send_email`. `add-admin-rbac` menyediakan `require_admin`. Acuan: FR-ADM-WRK/JOB/GDS-* di PRD Dashboard.

## Goals / Non-Goals

**Goals:**
- State moderasi + soft-delete pada keempat iklan; suspend per-iklan single/bulk dengan alasan + bukti + notifikasi.
- Listing admin dengan search/filter/sort/pagination + export CSV + foto popup.
- Memperbaiki insert field yang selama ini hilang agar data moderasi lengkap.

**Non-Goals:**
- Edit data iklan oleh admin; penyelarasan `lokasi` ke region; pelatihan lifecycle; aduan.

## Decisions

### D1 — `moderation_status` + `deleted_at`, pertahankan flag existing
Tambah `moderation_status TEXT NOT NULL DEFAULT 'active'` (`CHECK active|suspended_temp|suspended_permanent`) dan `deleted_at TIMESTAMPTZ NULL`. Flag domain existing (`is_active`/`is_sold`) **tetap** sebagai status fungsional pemilik (aktif/terjual); `moderation_status` adalah status moderasi admin yang **orthogonal**. List publik memfilter `moderation_status='active' AND deleted_at IS NULL` (+ flag existing). Suspend mengubah `moderation_status`, bukan menghapus.

### D2 — Suspend per-iklan dengan bukti, pola seragam `account_suspension`
Mengikuti pola `auth.account_suspension` (sudah ada `evidence_object_key` dari KYC Q1). Tabel `iklan_suspension` per schema iklan: `(id, iklan_id, is_permanent, reason NOT NULL, evidence_object_key, expires_at, created_by, created_at)`. Bukti via `StorageClient` kategori baru `iklan-suspension-evidence` (≤5MB, JPEG/PNG/PDF — sama batas `suspension-evidence`). Suspend sementara mengisi `expires_at`; permanen tidak. Bulk = satu permintaan berisi daftar `iklan_id` + satu alasan + satu bukti (atau bukti per item — lihat Open Questions).

### D3 — Endpoint admin terpisah dari endpoint publik
Sub-router `/api/v1/admin/{vertikal}` diproteksi `require_admin`. List admin menerima query: `q` (search judul/kode/pembuat), `status` (filter/sort), `limit`/`offset`. Search judul/kode via `ILIKE`/match; "pembuat" via `poster_id`/`seller_id` (atau username bila di-join — lihat Open Questions). Endpoint publik existing tidak berubah (kecuali surface foto). Memisahkan admin menjaga IDOR & kontrak publik tetap stabil.

### D4 — Export CSV streaming mengikuti filter aktif
`GET /admin/{vertikal}/export.csv` menghasilkan `text/csv` (header + baris) mengikuti filter/search yang sama dengan listing. Untuk dataset besar, query dibatasi/di-stream; kolom CSV mengikuti kolom tabel User Story. Tidak memuat data sensitif (NIK/KTP tidak ada di iklan — dijamin oleh domain).

### D5 — Surface foto untuk popup
`iklan_barang_bekas.foto_urls` sudah ada sebagai kolom DB tetapi tidak di create-DTO/response — kini di-plumbing penuh (insert + response). Vertikal "pekerja"/"pekerjaan" memerlukan field foto pekerjaan: tambah `foto_urls TEXT[]` setara bila User Story menuntut "Foto Pekerjaan" (FR-ADM-WRK-01/JOB-01). Penyimpanan tetap object key/URL via StorageClient (konsisten dgn avatar/dokumen).

### D6 — Perbaikan insert field yang hilang
Service layer saat ini meng-INSERT subset field. Perbaiki agar `lokasi`, `gaji_*`/`tarif_*`/`harga`, `tanggal_*` (pelatihan) tersimpan. Ini bug-fix yang menyertai moderasi karena data lengkap diperlukan admin (mis. menampilkan upah/lokasi di tabel).

### D7 — Kepatuhan standar Phase 1.5
Envelope `ApiResponse`, error RFC 9457-inspired, IDOR→404 & ownership di query, soft-delete, propagasi `request_id`, snake_case DB, `created_at`/`updated_at`. Admin read-only kecuali suspend.

### D8 — `ModerationStatus` sebagai Rust enum (type safety)
Kolom `moderation_status` di DB tetap `TEXT CHECK (...)`, tetapi di Rust direpresentasikan sebagai enum:
```rust
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModerationStatus { Active, SuspendedTemp, SuspendedPermanent }
```
Konversi DB → Rust via `ModerationStatus::parse()` pada boundary `row_to_entity()`. Serialisasi JSON tetap `"active"` / `"suspended_temp"` / `"suspended_permanent"`. Ini membuat illegal state **unrepresentable** di compile-time — kode Rust tidak dapat mengirim string arbitrer ke DTO response.

### D9 — Auto-expire temporary suspensions
Scheduler `expire_temporary_suspensions()` di-repository mengembalikan `suspended_temp` → `active` saat `expires_at <= now()` melalui query:
```sql
UPDATE iklan_... SET moderation_status='active', updated_at=now()
WHERE moderation_status='suspended_temp'
  AND id IN (SELECT iklan_id FROM iklan_suspension
             WHERE is_permanent=false AND expires_at IS NOT NULL AND expires_at <= now())
  AND deleted_at IS NULL
```
Dipanggil oleh cron/tokio interval di level application service. Hanya `suspended_temp` yang terpengaruh; `suspended_permanent` **tidak pernah** kembali aktif — iklan permanen yang di-suspend tetap `suspended_permanent` selamanya.

### D10 — Cooldown 3 hari untuk poster dengan suspend permanen
Pengguna reguler yang iklannya di-suspend permanen **tidak dapat membuat iklan baru** selama 3 hari (`is_poster_in_cooldown()` check di `create()`). Query:
```sql
SELECT EXISTS(SELECT 1 FROM iklan_suspension
              WHERE is_permanent=true
                AND created_at > now() - INTERVAL '3 days'
                AND iklan_id IN (SELECT id FROM iklan WHERE poster_id=$1))
```
Cooldown dihitung dari waktu **suspension record** dibuat (`created_at`), bukan dari waktu `expires_at`. Setelah 3 hari lewat, `EXISTS` mengembalikan `false` dan user dapat membuat iklan lagi.

### D11 — Single-query admin listing dengan `COUNT(*) OVER()`
Admin list sebelumnya membutuhkan 2 round-trip (COUNT + SELECT). Dioptimalkan menjadi 1 query menggunakan PostgreSQL window function:
```sql
SELECT ..., COUNT(*) OVER() AS total_rows
FROM iklan_...
WHERE ...
ORDER BY created_at DESC LIMIT $1 OFFSET $2
```
Nilai `total_rows` diekstrak dari kolom pertama hasil query — mengeliminasi query COUNT terpisah tanpa mengubah hasil.

## Risks / Trade-offs

- **Empat service serupa = duplikasi** → terima duplikasi terkendali (pola sudah identik); pertimbangkan util bersama untuk CSV/suspension bila perlu, tanpa memaksa abstraksi lintas-domain.
- **Bulk suspend & notifikasi massal** → gunakan `send_bulk`/iterasi; tangani kegagalan parsial (sebagian iklan ter-suspend) dengan melaporkan hasil per item.
- **Soft-delete vs hard-delete existing** → `DELETE /{id}` pemilik tetap; admin memakai suspend/soft-delete. Perlu konsistensi agar list publik menyaring keduanya.
- **CSV besar** → batasi/stream; dokumentasikan batas bila ada (jangan diam-diam memotong).
- **Field foto pekerja/pekerjaan** → menambah kolom baru; pastikan migrasi nullable & alur upload via StorageClient.

## Migration Plan

1. Migrasi keempat schema iklan: tambah `moderation_status` + `deleted_at` + index; tabel `iklan_suspension`; (bila perlu) kolom `foto_urls` untuk pekerja/pekerjaan (up + down).
2. Tambah kategori `iklan-suspension-evidence` di `StorageClient`.
3. Perbaiki INSERT field yang hilang + surface `foto_urls` di create/response.
4. Implementasi listing admin (search/filter/sort/pagination) + export CSV per vertikal.
5. Implementasi suspend single/bulk + bukti presigned + penyimpanan `iklan_suspension` + notifikasi (email + in-app).
6. Filter list publik agar mengecualikan `suspended`/`deleted`.
7. Wire `require_admin`, `StorageClient`, `NotificationClient` ke router admin iklan di `rejki-app`.
8. Rollback: migrasi turun drop kolom/tabel baru; endpoint admin dilepas; list publik kembali ke flag existing.

## Resolved Questions

- **Bukti wajib pada suspend** (selaras KYC Q1 & PRD): suspend WAJIB menyertakan alasan + bukti (gambar/PDF ≤5MB) demi audit & sengketa.
- **Admin read-only** atas data iklan: hanya suspend yang diizinkan, tidak ada edit (FR-ADM-*-08).

## Open Questions

- **"Barang Bekas Gratis" vs harga**: User Story menamai menu "Barang Bekas **Gratis**" dengan kolom Jumlah & Lokasi Pengambilan & status `Sudah Diambil`, sementara entity backend punya `harga` & `is_sold`. Perlu konfirmasi pemilik apakah model bergeser ke gratis/donasi (mengganti `harga`) atau tetap jual-beli. Sementara: pertahankan `harga`, tambah field yang diminta sebagai aditif.
- **Bukti per-item vs satu bukti untuk bulk**: apakah bulk suspend memakai satu bukti untuk semua atau bukti per iklan? Default usulan: satu alasan+bukti per permintaan bulk; dapat diperhalus.
- **"Pembuat" pada search**: apakah pencarian pembuat berdasarkan `poster_id`/`seller_id` (UUID) atau nama/username (perlu join ke user-service)? Perlu keputusan; default: cari by ID, opsional join username.
- **Pelatihan**: kolom moderasi dasar di sini vs lifecycle penuh di `add-pelatihan-enrollment-badge` — koordinasi agar tidak duplikat migrasi.
