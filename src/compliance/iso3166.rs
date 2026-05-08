//! ISO 3166-1 alpha-2 country codes.

/// Validate an ISO 3166-1 alpha-2 country code.
pub fn is_valid_country(code: &str) -> bool {
    COUNTRY_CODES.contains(&code)
}

/// Lookup country name by alpha-2 code.
pub fn country_name(code: &str) -> Option<&'static str> {
    COUNTRIES
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, name)| *name)
}

const COUNTRIES: &[(&str, &str)] = &[
    ("AR", "Argentina"),
    ("BO", "Bolivia"),
    ("BR", "Brazil"),
    ("CA", "Canada"),
    ("CL", "Chile"),
    ("CO", "Colombia"),
    ("CR", "Costa Rica"),
    ("CU", "Cuba"),
    ("DE", "Germany"),
    ("DO", "Dominican Republic"),
    ("EC", "Ecuador"),
    ("ES", "Spain"),
    ("FR", "France"),
    ("GB", "United Kingdom"),
    ("GT", "Guatemala"),
    ("HN", "Honduras"),
    ("JP", "Japan"),
    ("KR", "South Korea"),
    ("MX", "Mexico"),
    ("NI", "Nicaragua"),
    ("PA", "Panama"),
    ("PE", "Peru"),
    ("PY", "Paraguay"),
    ("SV", "El Salvador"),
    ("US", "United States"),
    ("UY", "Uruguay"),
    ("VE", "Venezuela"),
];

const COUNTRY_CODES: &[&str] = &[
    "AR", "BO", "BR", "CA", "CL", "CO", "CR", "CU", "DE", "DO", "EC", "ES", "FR", "GB", "GT", "HN",
    "JP", "KR", "MX", "NI", "PA", "PE", "PY", "SV", "US", "UY", "VE",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_codes() {
        assert!(is_valid_country("CL"));
        assert!(is_valid_country("US"));
        assert!(is_valid_country("AR"));
    }

    #[test]
    fn invalid_codes() {
        assert!(!is_valid_country("XX"));
        assert!(!is_valid_country("cl")); // case sensitive
        assert!(!is_valid_country(""));
    }

    #[test]
    fn lookup_name() {
        assert_eq!(country_name("CL"), Some("Chile"));
        assert_eq!(country_name("XX"), None);
    }
}
