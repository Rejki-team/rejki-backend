# Proposal: CEO Analytics Endpoint (W3D-12)

| | |
|---|---|
| **Change ID** | `ws-ceo-analytics` |
| **Wave** | W3D-12 |
| **Priority** | P2 |
| **Status** | Draft |
| **Tanggal** | 2026-06-22 |

## Latar Belakang

Rejki CEO Mobile membutuhkan layer analytics read-only untuk eksekutif memantau kesehatan bisnis platform. Saat ini seluruh data mentah tersedia di database transaksional, tapi tidak ada endpoint agregasi yang siap pakai.

## Tujuan

1. Menyediakan endpoint `/api/v1/insights/*` untuk dashboard eksekutif
2. Menggunakan materialized view agar tidak membebani DB transaksional
3. Melindungi akses dengan RBAC role Executive (rank 90)
4. Auto-refresh 30 menit + manual trigger dari CEO Mobile
5. Menyediakan data canvassing untuk menentukan wilayah prioritas ekspansi

## Dependencies

- W3A-03 (region_id di 4 iklan) ✅
- W3B-05 (Multi-tier RBAC) ✅
- Perlu penambahan role Executive (rank 90) di auth-service

## Metrik yang Disajikan

- **Users**: total, baru, aktif, terverifikasi, growth rate, conversion funnel
- **Iklan**: per vertikal (total, aktif, baru, suspended, % terjual, trend)
- **Geo**: sebaran per provinsi, supply-demand ratio, canvassing score
- **Engagement**: chat volume, notifikasi terkirim/dibaca
- **Canvassing**: ranking prioritas wilayah + rekomendasi
