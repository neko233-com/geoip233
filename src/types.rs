use maxminddb::geoip2;
use std::fmt;

/// GeoIP lookup result for a city-level database.
#[derive(Debug, Clone, Default)]
pub struct GeoIpCity {
    /// Two-letter country code (e.g., "US", "CN").
    pub country: Option<String>,
    /// Country name in English.
    pub country_name: Option<String>,
    /// Country code for geolocation.
    pub country_code: Option<String>,
    /// City name in English.
    pub city: Option<String>,
    /// Subdivision (state/province) code.
    pub region_code: Option<String>,
    /// Subdivision (state/province) name.
    pub region_name: Option<String>,
    /// Postal/ZIP code.
    pub postal_code: Option<String>,
    /// Latitude.
    pub latitude: Option<f64>,
    /// Longitude.
    pub longitude: Option<f64>,
    /// Timezone (e.g., "America/New_York").
    pub timezone: Option<String>,
    /// Continent code (e.g., "NA", "AS").
    pub continent: Option<String>,
    /// Continent name.
    pub continent_name: Option<String>,
}

impl GeoIpCity {
    /// Convert from maxminddb's GeoIP2 City record.
    pub(crate) fn from_mmdb(city: geoip2::City<'_>) -> Self {
        let country = city.country.as_ref();
        let location = city.location.as_ref();
        let subdivisions = city.subdivisions.as_ref().and_then(|s| s.first());

        Self {
            country: country.and_then(|c| c.iso_code.map(|s| s.to_owned())),
            country_name: country
                .and_then(|c| c.names.as_ref())
                .and_then(|n| n.get("en").map(|s| s.to_string())),
            country_code: country.and_then(|c| c.iso_code.map(|s| s.to_owned())),
            city: city
                .city
                .as_ref()
                .and_then(|c| c.names.as_ref())
                .and_then(|n| n.get("en").map(|s| s.to_string())),
            region_code: subdivisions.and_then(|s| s.iso_code.map(|st| st.to_owned())),
            region_name: subdivisions
                .and_then(|s| s.names.as_ref())
                .and_then(|n| n.get("en").map(|s| s.to_string())),
            postal_code: city
                .postal
                .as_ref()
                .and_then(|p| p.code.map(|s| s.to_owned())),
            latitude: location.and_then(|l| l.latitude),
            longitude: location.and_then(|l| l.longitude),
            timezone: location.and_then(|l| l.time_zone.map(|s| s.to_owned())),
            continent: city
                .continent
                .as_ref()
                .and_then(|c| c.code.map(|s| s.to_owned())),
            continent_name: city
                .continent
                .as_ref()
                .and_then(|c| c.names.as_ref())
                .and_then(|n| n.get("en").map(|s| s.to_string())),
        }
    }

    /// Get country code, defaulting to "Unknown".
    pub fn country_str(&self) -> &str {
        self.country.as_deref().unwrap_or("Unknown")
    }

    /// Get city name, defaulting to "Unknown".
    pub fn city_str(&self) -> &str {
        self.city.as_deref().unwrap_or("Unknown")
    }

    /// Get coordinates as (latitude, longitude) if available.
    pub fn coordinates(&self) -> Option<(f64, f64)> {
        match (self.latitude, self.longitude) {
            (Some(lat), Some(lon)) => Some((lat, lon)),
            _ => None,
        }
    }
}

impl fmt::Display for GeoIpCity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}, {}", self.city_str(), self.country_str())
    }
}

/// Simplified GeoIP result with only country information.
#[derive(Debug, Clone, Default)]
pub struct GeoIpCountry {
    /// Two-letter country code.
    pub code: Option<String>,
    /// Country name.
    pub name: Option<String>,
    /// Continent code.
    pub continent: Option<String>,
}

impl GeoIpCountry {
    pub fn code_str(&self) -> &str {
        self.code.as_deref().unwrap_or("Unknown")
    }

    pub fn name_str(&self) -> &str {
        self.name.as_deref().unwrap_or("Unknown")
    }
}

impl fmt::Display for GeoIpCountry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.name_str(), self.code_str())
    }
}

/// Metadata about a loaded GeoIP database.
#[derive(Debug, Clone)]
pub struct GeoIpMetadata {
    pub database_type: String,
    pub description: Option<String>,
    pub build_epoch: Option<u64>,
    pub node_count: Option<u64>,
}
