# Design: ws-housekeeping

> Workstream ini bersifat **housekeeping murni** (dokumentasi + arsip). Karena tidak ada
> keputusan arsitektur kode, design doc ini ringkas — fokus pada **rasionale keputusan
> status** dan **kriteria verifikasi**, bukan tradeoff implementasi.

## D1 — Kriteria penandaan checklist "✓ Selesai" vs "Pending"

**Keputusan:** Item checklist hanya boleh ditandai "Selesai" jika **keberadaan implementasi
terverifikasi di kode/repositori** (file ada, symbol ada, workflow ada), BUKAN hanya karena
didokumentasikan di Phase 1.5. Sebaliknya, item tetap "Pending" jika salah satu:
- Tidak ada file/symbol yang merepresentasikannya, atau
- Hanya terimplementasi parsial (mis. rate limiting hanya di satu service).

**Rasionale:** Anomali checklist yang ditemukan (label "Pending" pada item yang sebenarnya sudah
ada) bersumber dari checklist yang ditulis berdasarkan rencana, bukan kondisi kode. CLAUDE.md §0.1
melarang asumsi tanpa dasar — oleh karena itu setiap label didahului verifikasi eksplisit (task
section 1.1–1.5).

## D2 — Mengapa dua change diarsipkan, dua change tidak

| Change | Status task | Tindakan | Alasan |
|---|---|---|---|
| `add-storage-service-spec` | 100% `[x]`, clippy pass | **Arsip** | Selesai penuh, capability di-wire di rejki-app |
| `fix-phase2-code-review-findings` | 45/46, sisa = arsip itu sendiri | **Arsip** | Selesai fungsional, task tersisa hanya meta-arsip |
| `fix-auth-service-code-review-findings` | 87/91, sisa = 4 task coverage | **Tidak arsip** | Sisa task coverage = kepemilikan P2; arsip prematur akan mengaburkan transfer |
| `fix-user-service-code-review-findings` | 49/60, sisa = 11 task (mayoritas test) | **Tidak arsip** | Sama; ditambah ada 1 task komunikasi tim non-code |

**Risiko tidak-mengarsipkan:** kedua change aktif tetap menempati daftar aktif. **Mitigasi:**
anotasi transfer kepemilikan eksplisit (D3) membuat statusnya jelas — bukan "terbengkalai" melainkan
"menunggu P2 menutup coverage".

## D3 — Mekanisme anotasi transfer kepemilikan task

**Keputusan:** Task coverage yang deferred-by-design ditandai dengan format konsisten pada
`tasks.md`:

```
- [ ] <deskripsi asli> (deferred → `ws-testing-coverage` (P2) section <N> — <alasan>)
```

**Rasionale:**
- Tetap `[ ]` (bukan `[x]`) → tidak mengklaim selesai. CLAUDE.md §8 melarang menandai task selesai
  sebelum terimplementasi & terverifikasi.
- Pointer eksplisit ke P2 section → mencegah duplikasi pekerjaan dan deadlock kepemilikan
  ("siapa yang nulis test ini?").
- Alasan tercatat → traceable mengapa didefer (mis. "integration tests live in rejki-app",
  "tooling nightly tidak terinstall").

**Pemetaan ke P2:**
- P2 section 1 = unit test per-service crate (auth, user, dst.)
- P2 section 2 = integration test per-service + rejki-app integration
- P2 section 3 = coverage gate CI + tooling (llvm-cov/tarpaulin)

## D4 — Tidak ada perubahan kode: implikasi

Karena workstream ini murni dokumentasi + arsip:
- **Tidak perlu** `cargo fmt` / `clippy` / `sqlx prepare` (tidak ada kode Rust berubah).
- **Tidak perlu** migrasi DB.
- **Tidak perlu** update Swagger (tidak ada perubahan API/endpoint).
- Sanity check `cargo check --workspace` tetap dijalankan (task 5) untuk memastikan arsip
  direktori tidak merusak workspace manifest — kemungkinan besar no-op, tapi murah untuk diverifikasi.
- `cargo clippy -p storage-service` di task 2.1 adalah **verifikasi prasyarat** sebelum arsip,
  bukan perubahan kode.

## D5 — Urutan eksekusi (anti-bottleneck)

1. Section 1 dulu (verifikasi + koreksi checklist) — dasar kebenaran status.
2. Section 3 & 4 (anotasi transfer) — sebelum arsip, agar status dua change aktif jelas.
3. Section 2 (arsip dua change siap) — terakhir, setelah status seluruh change stabil.
4. Section 5 (verifikasi akhir).

Urutan ini memastikan: jika ada perubahan rencana di tengah jalan (mis. ditemukan task siap-arsip
ternyata belum selesai), anotasi transfer sudah siap dan checklist sudah jujur.
