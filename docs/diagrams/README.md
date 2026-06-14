# Diagram PlantUML — Rejki Web Dashboard & Ekosistem

Sumber diagram (`.puml`) untuk PRD Rejki Web Dashboard ([../prd/prd-dashboard.md](../prd/prd-dashboard.md)) dan ekosistem ([../prd/rejki-prd.md](../prd/rejki-prd.md)). Disimpan sebagai **sumber teks** (bukan gambar) agar versionable dan mudah ditinjau di PR.

| File | Jenis | Isi |
|---|---|---|
| `ecosystem-context.puml` | C4 Context | 3 klien (Mobile, Dashboard, CEO) + backend + infra |
| `dashboard-container.puml` | C4 Container | SPA Vue ↔ endpoint admin ↔ service backend |
| `rbac-account-status.puml` | State | State machine status akun + posisi role admin |
| `kyc-verification-sequence.puml` | Sequence | Admin review KYC: mask → click-to-view (audit) → approve/reject → notifikasi → purge dok |
| `iklan-moderation-sequence.puml` | Sequence | Suspend per-iklan: ceklis → alasan+bukti → notifikasi |
| `pelatihan-lifecycle.puml` | State | 7 status pelatihan + auto-approve admin vs review user |
| `training-enrollment-badge.puml` | Sequence | Konfirmasi pelatihan (bukti transfer) → badge/sertifikat |
| `report-aduan-flow.puml` | Sequence | User lapor (Mobile) → admin tindak lanjut (Dashboard) |
| `corporate-comms-broadcast.puml` | Sequence | Admin buat/edit artikel → broadcast notifikasi |
| `dashboard-navigation.puml` | Komponen | Peta sidebar/menu/sub-menu sesuai User Story |

## Cara render

Butuh [PlantUML](https://plantuml.com) (Java + `plantuml.jar`, atau Docker, atau ekstensi editor).

```bash
# Satu file → PNG/SVG
plantuml -tsvg docs/diagrams/ecosystem-context.puml

# Semua file
plantuml -tsvg docs/diagrams/*.puml

# Cek sintaks tanpa render
plantuml -checkonly docs/diagrams/*.puml
```

Alternatif tanpa instalasi: tempel isi `.puml` ke <https://www.plantuml.com/plantuml>.

> Diagram C4 memakai sprite `C4-PlantUML` via `!include` URL. Jika offline, ganti dengan notasi `component`/`rectangle` biasa (lihat komentar di tiap file C4) atau gunakan cache lokal `C4_Context.puml`.

## Keterlacakan

Setiap diagram mereferensikan FR/area di [prd-dashboard.md](../prd/prd-dashboard.md) dan change OpenSpec terkait (`add-admin-rbac`, `extend-iklan-moderation`, `add-pelatihan-enrollment-badge`, `add-content-reports`, `add-corporate-comms`, `add-rejki-web-dashboard`) pada blok catatan di dalam file.
