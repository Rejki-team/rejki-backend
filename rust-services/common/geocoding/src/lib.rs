//! `GeocodingClient` — konversi alamat teks → koordinat lat/lng (F-1, PRD §5.11.2).
//!
//! Provider: Nominatim (OpenStreetMap) API publik — dipilih karena gratis, tanpa API key,
//! tanpa risiko biaya/lisensi (data ODbL), dan TIDAK butuh infra baru (self-hosting Nominatim
//! butuh planet OSM extract + service Docker terpisah — overkill untuk skala saat ini).
//!
//! ponytail: usage policy Nominatim publik membatasi ~1 req/detik & wajib header User-Agent —
//! cukup untuk pola pakai di sini (geocode sekali per create/update alamat, bukan batch masif).
//! Upgrade ke self-hosted Nominatim atau provider berbayar bila traffic geocoding jadi
//! signifikan (mis. import massal / bulk re-geocode).

use async_trait::async_trait;
use serde::Deserialize;

const NOMINATIM_BASE_URL: &str = "https://nominatim.openstreetmap.org/search";
const USER_AGENT: &str = "RejkiPlatform/1.0 (contact: dev@rejki.id)";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Coordinates {
    pub latitude: f64,
    pub longitude: f64,
}

/// Alamat yang akan di-geocode, dengan fallback 2-tingkat sesuai PRD §5.11.2:
/// (1) `address_line` + kelurahan/kecamatan/kota, (2) kelurahan/kecamatan/kota saja.
#[derive(Debug, Clone, Default)]
pub struct GeocodeInput {
    pub address_line: Option<String>,
    pub village_name: Option<String>,
    pub district_name: Option<String>,
    pub regency_name: Option<String>,
}

impl GeocodeInput {
    fn join_parts(parts: &[Option<&str>]) -> Option<String> {
        let joined: Vec<&str> = parts.iter().copied().flatten().collect();
        if joined.is_empty() {
            None
        } else {
            Some(joined.join(", "))
        }
    }

    /// Tingkat 1: alamat lengkap + kelurahan + kecamatan + kota.
    fn full_query(&self) -> Option<String> {
        Self::join_parts(&[
            self.address_line.as_deref(),
            self.village_name.as_deref(),
            self.district_name.as_deref(),
            self.regency_name.as_deref(),
        ])
    }

    /// Tingkat 2 (fallback): kelurahan + kecamatan + kota saja (tanpa alamat lengkap).
    fn fallback_query(&self) -> Option<String> {
        Self::join_parts(&[
            self.village_name.as_deref(),
            self.district_name.as_deref(),
            self.regency_name.as_deref(),
        ])
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GeocodingClientError {
    #[error("geocoding provider unavailable")]
    Unavailable,
}

#[async_trait]
pub trait GeocodingClient: Send + Sync {
    /// Geocode alamat dengan fallback 2-tingkat. `Ok(None)` bila kedua tingkat sama-sama
    /// tidak menemukan hasil — BUKAN error, pemanggil wajib simpan tanpa koordinat
    /// (degradasi anggun, jangan block create/update — §4.5 backend).
    async fn geocode(
        &self,
        input: &GeocodeInput,
    ) -> Result<Option<Coordinates>, GeocodingClientError>;
}

#[derive(Deserialize)]
struct NominatimResult {
    lat: String,
    lon: String,
}

fn parse_first_result(results: &[NominatimResult]) -> Option<Coordinates> {
    let r = results.first()?;
    Some(Coordinates {
        latitude: r.lat.parse().ok()?,
        longitude: r.lon.parse().ok()?,
    })
}

pub struct NominatimGeocodingClient {
    http: reqwest::Client,
}

impl NominatimGeocodingClient {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .user_agent(USER_AGENT)
                .build()
                .expect("gagal membangun HTTP client geocoding"),
        }
    }

    async fn query(&self, q: &str) -> Result<Option<Coordinates>, GeocodingClientError> {
        let resp = self
            .http
            .get(NOMINATIM_BASE_URL)
            .query(&[
                ("q", q),
                ("format", "json"),
                ("limit", "1"),
                ("countrycodes", "id"),
            ])
            .send()
            .await
            .map_err(|e| {
                tracing::warn!(error = ?e, query = %q, "geocoding request gagal");
                GeocodingClientError::Unavailable
            })?;

        let results: Vec<NominatimResult> = resp.json().await.map_err(|e| {
            tracing::warn!(error = ?e, query = %q, "gagal parse respons geocoding");
            GeocodingClientError::Unavailable
        })?;

        Ok(parse_first_result(&results))
    }
}

impl Default for NominatimGeocodingClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl GeocodingClient for NominatimGeocodingClient {
    async fn geocode(
        &self,
        input: &GeocodeInput,
    ) -> Result<Option<Coordinates>, GeocodingClientError> {
        if let Some(q) = input.full_query() {
            if let Some(coords) = self.query(&q).await? {
                return Ok(Some(coords));
            }
        }
        if let Some(q) = input.fallback_query() {
            if let Some(coords) = self.query(&q).await? {
                tracing::info!(query = %q, "geocoding berhasil via fallback tingkat 2");
                return Ok(Some(coords));
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(
        address: Option<&str>,
        village: Option<&str>,
        district: Option<&str>,
        regency: Option<&str>,
    ) -> GeocodeInput {
        GeocodeInput {
            address_line: address.map(String::from),
            village_name: village.map(String::from),
            district_name: district.map(String::from),
            regency_name: regency.map(String::from),
        }
    }

    #[test]
    fn test_full_query_given_all_fields_when_built_then_joins_in_order() {
        let i = input(
            Some("Jl. Test No. 1"),
            Some("Kel A"),
            Some("Kec B"),
            Some("Kota C"),
        );
        assert_eq!(
            i.full_query(),
            Some("Jl. Test No. 1, Kel A, Kec B, Kota C".to_string())
        );
    }

    #[test]
    fn test_fallback_query_given_all_fields_when_built_then_excludes_address_line() {
        let i = input(
            Some("Jl. Test No. 1"),
            Some("Kel A"),
            Some("Kec B"),
            Some("Kota C"),
        );
        assert_eq!(i.fallback_query(), Some("Kel A, Kec B, Kota C".to_string()));
    }

    #[test]
    fn test_full_query_given_no_fields_when_built_then_none() {
        let i = input(None, None, None, None);
        assert_eq!(i.full_query(), None);
        assert_eq!(i.fallback_query(), None);
    }

    #[test]
    fn test_fallback_query_given_only_village_when_built_then_partial_join() {
        let i = input(None, Some("Kel A"), None, Some("Kota C"));
        assert_eq!(i.fallback_query(), Some("Kel A, Kota C".to_string()));
    }

    #[test]
    fn test_parse_first_result_given_valid_lat_lon_when_parsed_then_coordinates() {
        let results = vec![NominatimResult {
            lat: "-6.200000".to_string(),
            lon: "106.816666".to_string(),
        }];
        let coords = parse_first_result(&results).unwrap();
        assert!((coords.latitude - (-6.2)).abs() < 1e-6);
        assert!((coords.longitude - 106.816666).abs() < 1e-6);
    }

    #[test]
    fn test_parse_first_result_given_empty_when_parsed_then_none() {
        assert!(parse_first_result(&[]).is_none());
    }

    #[test]
    fn test_parse_first_result_given_invalid_lat_when_parsed_then_none() {
        let results = vec![NominatimResult {
            lat: "not-a-number".to_string(),
            lon: "106.8".to_string(),
        }];
        assert!(parse_first_result(&results).is_none());
    }
}
