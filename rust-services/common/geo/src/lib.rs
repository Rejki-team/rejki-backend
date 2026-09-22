//! Fungsi jarak geografis reusable (F-1, PRD §7 radius filtering) — bounding-box pre-filter
//! (dipakai di WHERE query SQL, memanfaatkan index `(latitude, longitude)`) + Haversine untuk
//! jarak presisi (potongan akhir di application layer setelah bounding-box mempersempit baris,
//! Hazard #6/#7: BUKAN full-table scan + Haversine per baris).
//!
//! Dipakai oleh 4 service iklan (radius listing 2km/10km) dan akan dipakai ulang oleh geofence
//! 50m "Mulai Bekerja" (Kelompok 3, entity Lamaran Pekerjaan).
//!
//! ponytail: valid untuk latitude non-kutub (wilayah Indonesia ±11°..+6°) — `cos(lat)` mendekati
//! 0 di dekat kutub akan membuat `delta_lng` meledak; tidak di-guard karena tidak relevan untuk
//! skala/domain proyek ini.

/// Radius Bumi rata-rata (km) — standar untuk formula Haversine.
const EARTH_RADIUS_KM: f64 = 6371.0;
/// Jarak (km) per 1 derajat lintang — konstan (meridian).
const KM_PER_DEGREE_LAT: f64 = 111.32;

/// Parameter query radius (titik pusat + radius km) — dipakai repository listing 4 service
/// iklan untuk filter "dalam radius X km dari koordinat pengguna".
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RadiusQuery {
    pub lat: f64,
    pub lng: f64,
    pub radius_km: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundingBox {
    pub min_lat: f64,
    pub max_lat: f64,
    pub min_lng: f64,
    pub max_lng: f64,
}

/// Bounding box persegi di sekitar `(lat, lng)` yang membungkus lingkaran radius `radius_km`.
/// Dipakai sebagai pre-filter `WHERE latitude BETWEEN ... AND longitude BETWEEN ...` — index
/// scan pada composite index `(latitude, longitude)`, bukan sequential scan.
pub fn bounding_box(lat: f64, lng: f64, radius_km: f64) -> BoundingBox {
    let delta_lat = radius_km / KM_PER_DEGREE_LAT;
    let km_per_degree_lng = KM_PER_DEGREE_LAT * lat.to_radians().cos();
    let delta_lng = if km_per_degree_lng.abs() < f64::EPSILON {
        180.0 // di dekat kutub, longitude tidak lagi membatasi apa pun — tidak relevan di sini
    } else {
        radius_km / km_per_degree_lng
    };
    BoundingBox {
        min_lat: lat - delta_lat,
        max_lat: lat + delta_lat,
        min_lng: lng - delta_lng,
        max_lng: lng + delta_lng,
    }
}

/// Jarak great-circle (km) antara dua titik lat/lng — formula Haversine.
pub fn haversine_km(lat1: f64, lng1: f64, lat2: f64, lng2: f64) -> f64 {
    let d_lat = (lat2 - lat1).to_radians();
    let d_lng = (lng2 - lng1).to_radians();
    let a = (d_lat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (d_lng / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().asin();
    EARTH_RADIUS_KM * c
}

/// `true` bila `(lat, lng)` berada dalam radius `radius_km` dari `(center_lat, center_lng)`.
pub fn within_radius_km(
    center_lat: f64,
    center_lng: f64,
    lat: f64,
    lng: f64,
    radius_km: f64,
) -> bool {
    haversine_km(center_lat, center_lng, lat, lng) <= radius_km
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 1 derajat longitude di ekuator (lat=0) ≈ 111.32 km — nilai referensi umum.
    #[test]
    fn test_haversine_km_given_one_degree_longitude_at_equator_when_computed_then_approx_111km() {
        let d = haversine_km(0.0, 0.0, 0.0, 1.0);
        assert!((d - 111.19).abs() < 1.0, "jarak: {d}");
    }

    #[test]
    fn test_haversine_km_given_same_point_when_computed_then_zero() {
        assert_eq!(haversine_km(-6.2, 106.8, -6.2, 106.8), 0.0);
    }

    /// Jakarta (-6.2088, 106.8456) ke Bandung (-6.9175, 107.6191) ≈ 115-120 km (nilai referensi umum).
    #[test]
    fn test_haversine_km_given_jakarta_to_bandung_when_computed_then_approx_115km() {
        let d = haversine_km(-6.2088, 106.8456, -6.9175, 107.6191);
        assert!((100.0..135.0).contains(&d), "jarak: {d}");
    }

    #[test]
    fn test_bounding_box_given_point_when_computed_then_center_within_box() {
        let bb = bounding_box(-6.2, 106.8, 2.0);
        assert!(bb.min_lat < -6.2 && -6.2 < bb.max_lat);
        assert!(bb.min_lng < 106.8 && 106.8 < bb.max_lng);
    }

    /// Titik pas di tepi radius (Haversine) harus tetap masuk bounding box (box selalu >= lingkaran).
    #[test]
    fn test_bounding_box_given_point_at_radius_edge_when_computed_then_still_within_box() {
        let (center_lat, center_lng, radius_km) = (-6.2, 106.8, 2.0);
        let bb = bounding_box(center_lat, center_lng, radius_km);
        // Titik 2km lurus ke utara dari center — tepat di edge radius.
        let edge_lat = center_lat + radius_km / KM_PER_DEGREE_LAT;
        assert!(edge_lat <= bb.max_lat + 1e-9);
    }

    #[test]
    fn test_within_radius_km_given_point_inside_when_checked_then_true() {
        // ~1km ke utara dari center, radius 2km → di dalam.
        let d_lat = 1.0 / KM_PER_DEGREE_LAT;
        assert!(within_radius_km(-6.2, 106.8, -6.2 + d_lat, 106.8, 2.0));
    }

    #[test]
    fn test_within_radius_km_given_point_outside_when_checked_then_false() {
        // ~5km ke utara dari center, radius 2km → di luar.
        let d_lat = 5.0 / KM_PER_DEGREE_LAT;
        assert!(!within_radius_km(-6.2, 106.8, -6.2 + d_lat, 106.8, 2.0));
    }
}
