# Analisis Gap — Rejki Web Dashboard (Backend ↔ Web)

| | |
|---|---|
| **Dokumen** | Analisis Arsitektur (Technical Architect) |
| **Tanggal** | 2026-06-15 |
| **Acuan User Story** | Pemilik produk — Rejki Web Dashboard (admin), per halaman/menu |
| **Metode** | Penelusuran **kode aktual** `rust-services/` (bukan klaim dokumen). Setiap status disitasi ke baris kode. |
| **Status** | Spesifikasi gap (snapshot audit 2026-06-15). **Update 2026-06-15:** seluruh 6 gap backend **DONE & terverifikasi** (44 test live). Narasi temuan di bawah dipertahankan sebagai rekam audit; kolom Status per-baris ditandai ✅ RESOLVED. |

> **Tujuan dokumen.** Menjawab pertanyaan pemilik produk: *"Mana saja yang belum ada atas User Story Dashboard?"* —
> memisahkan pekerjaan **rejki-backend** vs **rejki-web**, berdasar bukti kode, tanpa asumsi.
> Dokumen ini mengoreksi klaim "✅ selesai" pada [prd-dashboard.md](prd/prd-dashboard.md) v0.2 §4/§8 yang
> tidak akurat saat dicek ke kode, dan menjadi sumber keterlacakan untuk 3 OpenSpec change baru.

---

## 1. Ringkasan Eksekutif

Mayoritas backend admin **memang sudah ada & terverifikasi** (moderasi 4 vertikal, pelatihan penuh,
report/aduan, corporate comms, login admin + RBAC, KYC review). Namun penelusuran kode menemukan
**6 gap nyata** yang menghalangi pemenuhan User Story:

- **1 halaman penuh tidak punya backend listing**: "Pengelolaan Pengguna → Daftar Pengguna".
- **2 kewajiban retensi PDP tidak ter-wire**: auto-hapus dokumen KTP/Selfie saat **penolakan KYC**
  dan saat **suspend permanen**.
- **1 aksi massal tidak ada**: bulk suspend **pengguna** (suspend iklan sudah bulk, pengguna belum).
- **Akses dokumen admin tidak ada**: admin tidak bisa membaca KTP/Selfie pengguna lain (click-to-view).
- **Beberapa field entity** belum sesuai User Story (Iklan Pekerja, Barang Bekas Gratis).

Bukti pendukung **tertulis di repo sendiri**: `openspec/changes/add-user-service-kyc/tasks.md` baris 36–38
menandai `[6.3 admin] Baca dokumen oleh admin: menunggu RBAC` dan `[6.4 trigger] Pemusnahan auto: menunggu event`.
RBAC kini sudah ada (`add-admin-rbac` selesai), sehingga blocker tersebut **kini dapat dibuka**.

---

## 2. Pemetaan Lengkap: User Story → Endpoint Nyata → Status → Pemilik

Legenda status: **✅** terverifikasi ada di kode · **🔧** sebagian · **🔴/📋** belum ada (gap).
Pemilik: **BE** = rejki-backend · **WEB** = rejki-web.

### 2.1 Auth, Profil, Navigasi

| Kapabilitas User Story | Endpoint/bukti kode nyata | Status | Pemilik | Change |
|---|---|---|---|---|
| Login admin email+password (inject DBA) | `POST /api/v1/auth/admin/login` — [auth-service/interface/handlers.rs:59](../rust-services/auth-service/src/interface/handlers.rs#L59) | ✅ | BE✓/WEB | `add-admin-rbac` / `add-rejki-web-dashboard` |
| RBAC `role` admin + default-deny | [common/auth-middleware/src/lib.rs:98](../rust-services/common/auth-middleware/src/lib.rs#L98); [jwt.rs:53](../rust-services/auth-service/src/infrastructure/jwt.rs#L53) | ✅ | BE✓ | `add-admin-rbac` |
| Profil admin (foto, nama, role) | `GET /api/v1/users/me` mengandung `role` — [user-service/dto.rs:8-21](../rust-services/user-service/src/application/dto.rs#L8-L21) | ✅ | BE✓/WEB | — / `add-rejki-web-dashboard` |
| Logout via menu profil | `POST /api/v1/auth/logout` | ✅ | BE✓/WEB | — |
| Sidebar collapsible + menu/sub-menu | murni UI | 📋 | WEB | `add-rejki-web-dashboard` |

### 2.2 Iklan Pekerja / Pekerjaan

| Kapabilitas | Bukti kode | Status | Pemilik | Change |
|---|---|---|---|---|
| List + paginasi admin | `GET /pekerja/admin/` — [mod.rs:58](../rust-services/iklan-pekerja-service/src/interface/mod.rs#L58) | ✅ | BE✓/WEB | `extend-iklan-moderation` |
| Foto popup (`foto_urls`) | entity `foto_urls` — [entity.rs:49](../rust-services/iklan-pekerja-service/src/domain/entity.rs#L49) | ✅ | BE✓/WEB | — |
| Search (q) + sort status | `AdminListQuery { q, status, sort_by, sort_dir }` — [dto.rs:60-68](../rust-services/iklan-pekerja-service/src/application/dto.rs#L60-L68) | ✅ | BE✓/WEB | — |
| Suspend bulk iklan + bukti | `iklan_ids: Vec<Uuid>` — [dto.rs:77-85](../rust-services/iklan-pekerja-service/src/application/dto.rs#L77-L85); `POST /pekerja/admin/suspend` | ✅ | BE✓/WEB | — |
| Export CSV | `GET /pekerja/admin/export.csv` — [handlers.rs:83](../rust-services/iklan-pekerja-service/src/interface/handlers.rs#L83) | ✅ | BE✓/WEB | — |
| **Kolom Pengalaman Kerja** | ❌ tak ada field di [entity.rs:40-54](../rust-services/iklan-pekerja-service/src/domain/entity.rs#L40-L54) | 🔴 | BE | **Gap #6** (tidak dicakup change ini — lihat §4) |
| **Kolom Jam Kerja** | ❌ tak ada field | 🔴 | BE | **Gap #6** |
| **Kolom Cara Menghubungi** | ❌ tak ada field | 🔴 | BE | **Gap #6** |

> Iklan Pekerjaan: pola sama; entity punya `tipe` (full/part-time) bukan **Jam Kerja** eksplisit — [entity.rs:49](../rust-services/iklan-pekerjaan-service/src/domain/entity.rs#L49).

### 2.3 Iklan Pelatihan (3 sub-halaman)

| Kapabilitas | Bukti kode | Status | Pemilik |
|---|---|---|---|
| Daftar Pelatihan: CRUD admin + auto-approve + review user + 7 status | `/pelatihan/admin/pelatihan*` — [mod.rs:80-91](../rust-services/iklan-pelatihan-service/src/interface/mod.rs#L80-L91) | ✅ | BE✓/WEB |
| Konfirmasi Pelatihan: enrollment + bukti transfer + review | `/pelatihan/admin/enrollments*` — [mod.rs:98-107](../rust-services/iklan-pelatihan-service/src/interface/mod.rs#L98-L107) | ✅ | BE✓/WEB |
| Badge Pelatihan: sertifikat + review | `/pelatihan/admin/badges*` — [mod.rs:109-112](../rust-services/iklan-pelatihan-service/src/interface/mod.rs#L109-L112) | ✅ | BE✓/WEB |

### 2.4 Iklan Barang Bekas Gratis

| Kapabilitas | Bukti kode | Status | Pemilik | Change |
|---|---|---|---|---|
| List + suspend + CSV + foto popup | `/barang/admin/*` — [mod.rs:58-63](../rust-services/iklan-barang-bekas-service/src/interface/mod.rs#L58-L63) | ✅ | BE✓/WEB | `extend-iklan-moderation` |
| **Model "Gratis" (Jenis Barang, Jumlah, Lokasi Pengambilan, status Sudah Diambil)** | entity model **jual-beli**: `harga`, `kondisi`, `is_sold` — [entity.rs:45-49](../rust-services/iklan-barang-bekas-service/src/domain/entity.rs#L45-L49) | 🔴 | BE | **Change C** |

### 2.5 Pengelolaan Pengguna (Daftar Pengguna)

| Kapabilitas | Bukti kode | Status | Pemilik | Change |
|---|---|---|---|---|
| **Daftar pengajuan verifikasi (tabel)** | ❌ user-service admin hanya `POST /admin/kyc/{id}/review` — [mod.rs:74-84](../rust-services/user-service/src/interface/mod.rs#L74-L84) | 🔴 | BE | **Change A** |
| NIK ter-mask (default) | `nik_masked` `xxx...{l4}` — [service.rs:488](../rust-services/user-service/src/application/service.rs#L488) | ✅ | BE✓ | — |
| **Admin click-to-view KTP/Selfie (teraudit)** | ❌ `/me/documents/{kind}` **self-only** — [mod.rs:64](../rust-services/user-service/src/interface/mod.rs#L64); `kyc/tasks.md:37` `[6.3 admin] menunggu RBAC` | 🔴 | BE | **Change A** |
| Tolak/Terima KYC + terkunci | `POST /users/admin/kyc/{id}/review` — [service.rs:222](../rust-services/user-service/src/application/service.rs#L222) | ✅ | BE✓/WEB | — |
| Notifikasi hasil (email + in-app) | `notify` + `notify_email` — [service.rs:290-295](../rust-services/user-service/src/application/service.rs#L290-L295) | ✅ | BE✓ | — |
| **Auto-hapus dokumen saat DITOLAK** | ❌ `review_kyc(approved=false)` tak panggil `purge_documents` — [service.rs:222-296](../rust-services/user-service/src/application/service.rs#L222-L296); method ada tapi "tanpa HTTP trigger auto" [service.rs:427-428](../rust-services/user-service/src/application/service.rs#L427-L428) | 🔴 | BE | **Change A** |
| **Suspend pengguna bulk (1/sebagian/sekaligus)** | ✅ **RESOLVED** `BulkSuspendInput { user_ids: Vec<Uuid>, .. }`; `POST /auth/admin/users/suspend` partial-success — [auth-service/dto.rs:141-155](../rust-services/auth-service/src/application/dto.rs#L141-L155), [interface/mod.rs:112-121](../rust-services/auth-service/src/interface/mod.rs#L112-L121) | ✅ | BE✓ | **Change B** ✅ |
| Suspend pengguna single + bukti ≤5MB | `POST /auth/admin/users/{id}/suspend` + `/evidence` — [auth-service/interface/mod.rs:112-121](../rust-services/auth-service/src/interface/mod.rs#L112-L121) | ✅ | BE✓ | — |
| **Auto-hapus dokumen saat suspend PERMANEN** | ✅ **RESOLVED** trigger `UserClient::purge_kyc_documents` saat permanent — [auth-service/service.rs](../rust-services/auth-service/src/application/service.rs); `kyc/tasks.md:36` `[6.4 trigger]` resolved | ✅ | BE✓ | **Change B** ✅ |

### 2.6 Pengelolaan Dukungan (Aduan)

| Kapabilitas | Bukti kode | Status | Pemilik |
|---|---|---|---|
| List + detail (presigned foto bukti) + review wajib action_note | `/reports/admin/*` — [mod.rs:50-54](../rust-services/report-service/src/interface/mod.rs#L50-L54); `ReviewReportInput.action_note` wajib — [dto.rs:76-83](../rust-services/report-service/src/application/dto.rs#L76-L83) | ✅ | BE✓/WEB |
| Search + sort + CSV | `ReportListQuery` + `/admin/export.csv` — [dto.rs:65-72](../rust-services/report-service/src/application/dto.rs#L65-L72) | ✅ | BE✓/WEB |

### 2.7 Corporate Communication

| Kapabilitas | Bukti kode | Status | Pemilik |
|---|---|---|---|
| List/Create/Edit/Delete + foto + broadcast | `/api/v1/admin/articles/*` — [mod.rs:41-47](../rust-services/corporate-comms-service/src/interface/mod.rs#L41-L47) | ✅ | BE✓/WEB |

---

## 3. Daftar Gap & Pemilik (ringkas)

| # | Gap | Severity | Pemilik | Change OpenSpec |
|---|---|---|---|---|
| 1 | Listing pengajuan KYC/pengguna admin tidak ada | 🔴 blokir 1 halaman | BE | `add-user-admin-management` (A) |
| 2 | Admin baca dokumen KTP/Selfie pengguna lain (teraudit) | 🔴 | BE | `add-user-admin-management` (A) |
| 3 | Auto-purge dokumen saat KYC ditolak | 🔴 PDP | BE | `add-user-admin-management` (A) |
| 4 | Bulk suspend pengguna | ✅ DONE | BE | `extend-user-suspension-bulk-purge` (B) — `POST /auth/admin/users/suspend`, notif email+in-app |
| 5 | Auto-purge dokumen saat suspend permanen | ✅ DONE (PDP) | BE | `extend-user-suspension-bulk-purge` (B) — trigger via `UserClient` |
| 6 | Field entity (barang-bekas → model gratis) | 🟠 | BE | `extend-barang-bekas-gratis-model` (C) |

> **Catatan Gap #6 (Iklan Pekerja/Pekerjaan).** Field `Pengalaman Kerja`, `Jam Kerja`, `Cara Menghubungi`
> belum ada di entity. Keputusan pemilik produk pada iterasi ini **memprioritaskan model Barang Bekas Gratis**
> (Change C). Field iklan pekerja/pekerjaan dicatat sebagai **open question** (lihat §5) — perlu konfirmasi
> apakah dipetakan dari field existing (mis. `keahlian`/`deskripsi`) atau menambah kolom baru.

---

## 4. Pemisahan Pekerjaan: Backend vs Web

**rejki-backend (3 change baru — spesifikasi di OpenSpec):**
- Change A `add-user-admin-management` — listing KYC, akses dokumen admin teraudit, auto-purge saat reject, CSV.
- Change B `extend-user-suspension-bulk-purge` — bulk suspend pengguna + purge saat permanen + notifikasi.
- Change C `extend-barang-bekas-gratis-model` — selaraskan entity ke model gratis/donasi.

**rejki-web (greenfield — `add-rejki-web-dashboard`, belum ada kode):**
- Seluruh SPA: scaffold, auth store + interceptor refresh-queue, route guard, layout sidebar,
  8 komponen reusable, 10 halaman. Halaman "Pengelolaan Pengguna" tergantung Change A & B.

**Kontrak API yang WAJIB dipatuhi UI (terverifikasi dari kode):**
- Envelope: `{ success, data, meta?, request_id }` — [common/errors/src/lib.rs:95-101](../rust-services/common/errors/src/lib.rs#L95-L101).
- Paginasi: query `limit`/`offset` (bukan `page`); meta balikan `{ page, per_page, total, total_pages }` — [lib.rs:137-153](../rust-services/common/errors/src/lib.rs#L137-L153).
- Query admin standar: `q, status, sort_by, sort_dir, limit, offset`.

---

## 5. Open Questions

1. **Field Iklan Pekerja/Pekerjaan** (Gap #6): `Pengalaman Kerja`, `Jam Kerja`, `Cara Menghubungi`,
   `Jam Kerja` pekerjaan — tambah kolom baru atau map dari existing? → **menunggu konfirmasi pemilik**.
2. **Bulk suspend response code**: `200` dengan per-item result (selaras pola `SuspendResponse` iklan
   yang sudah ada) vs `207 Multi-Status`. Rekomendasi: ikuti pola existing (`200` + array result) demi
   konsistensi internal; `207` adalah alternatif standar industri (lihat §6).
3. **Storage delete cross-service** untuk purge dari auth-service: apakah auth memanggil user-service
   (HTTP/in-process client) atau user-service expose endpoint internal purge-by-user. → diputuskan di Change B design.

---

## 6. Sumber & Rujukan

**Internal (kode):** seluruh sitasi `rust-services/**` di atas; [prd-dashboard.md](prd/prd-dashboard.md);
`openspec/changes/add-user-service-kyc/tasks.md` (baris 36–38).

**Eksternal (praktik standar, kredibel):**
- Bulk operations & partial success (HTTP 207 Multi-Status):
  [OneUptime — REST API Bulk Operations](https://oneuptime.com/blog/post/2026-01-27-rest-api-bulk-operations/view),
  [OneUptime — Partial Success in Bulk APIs](https://oneuptime.com/blog/post/2026-02-02-rest-bulk-api-partial-success/view),
  [Apidog — 207 Multi-Status](https://apidog.com/blog/status-code-207-multi-status/),
  [Zalando RESTful API Guidelines #127](https://github.com/zalando/restful-api-guidelines/issues/127).
- PII masking + audit logging: [Zuplo — Protect Sensitive Data in API Logs](https://zuplo.com/learning-center/protect-sensitive-data-in-api-logs).
- Otorisasi & sesi: [OWASP Authorization Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Authorization_Cheat_Sheet.html),
  [OWASP Session Management Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html).
- (Untuk rejki-web) Vue3+Pinia+axios refresh-token queue:
  [Robust REST API Client in Vue 3](https://obeydi-abbassi.me/blog/vue3-axios-pinia-rest-client/),
  [Vue 3 + TypeScript Best Practices 2025](https://eastondev.com/blog/en/posts/dev/20251124-vue3-typescript-best-practices/).

---

_Lihat juga: [prd-dashboard.md](prd/prd-dashboard.md) · [rejki-prd.md](prd/rejki-prd.md) · diagram di [diagrams/](diagrams/)._
