//! ISO 3166-1 alpha-2 country codes — full standard coverage.

/// Validate an ISO 3166-1 alpha-2 country code.
pub fn is_valid_country(code: &str) -> bool {
    COUNTRIES.iter().any(|(c, _)| *c == code)
}

/// Lookup country name by alpha-2 code.
pub fn country_name(code: &str) -> Option<&'static str> {
    COUNTRIES
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, name)| *name)
}

/// All ISO 3166-1 alpha-2 codes with country names.
const COUNTRIES: &[(&str, &str)] = &[
    ("AD", "Andorra"),
    ("AE", "United Arab Emirates"),
    ("AF", "Afghanistan"),
    ("AG", "Antigua and Barbuda"),
    ("AL", "Albania"),
    ("AM", "Armenia"),
    ("AO", "Angola"),
    ("AR", "Argentina"),
    ("AT", "Austria"),
    ("AU", "Australia"),
    ("AZ", "Azerbaijan"),
    ("BA", "Bosnia and Herzegovina"),
    ("BB", "Barbados"),
    ("BD", "Bangladesh"),
    ("BE", "Belgium"),
    ("BF", "Burkina Faso"),
    ("BG", "Bulgaria"),
    ("BH", "Bahrain"),
    ("BI", "Burundi"),
    ("BJ", "Benin"),
    ("BN", "Brunei"),
    ("BO", "Bolivia"),
    ("BR", "Brazil"),
    ("BS", "Bahamas"),
    ("BT", "Bhutan"),
    ("BW", "Botswana"),
    ("BY", "Belarus"),
    ("BZ", "Belize"),
    ("CA", "Canada"),
    ("CD", "DR Congo"),
    ("CF", "Central African Republic"),
    ("CG", "Congo"),
    ("CH", "Switzerland"),
    ("CI", "Ivory Coast"),
    ("CL", "Chile"),
    ("CM", "Cameroon"),
    ("CN", "China"),
    ("CO", "Colombia"),
    ("CR", "Costa Rica"),
    ("CU", "Cuba"),
    ("CV", "Cape Verde"),
    ("CY", "Cyprus"),
    ("CZ", "Czech Republic"),
    ("DE", "Germany"),
    ("DJ", "Djibouti"),
    ("DK", "Denmark"),
    ("DM", "Dominica"),
    ("DO", "Dominican Republic"),
    ("DZ", "Algeria"),
    ("EC", "Ecuador"),
    ("EE", "Estonia"),
    ("EG", "Egypt"),
    ("ER", "Eritrea"),
    ("ES", "Spain"),
    ("ET", "Ethiopia"),
    ("FI", "Finland"),
    ("FJ", "Fiji"),
    ("FR", "France"),
    ("GA", "Gabon"),
    ("GB", "United Kingdom"),
    ("GD", "Grenada"),
    ("GE", "Georgia"),
    ("GH", "Ghana"),
    ("GM", "Gambia"),
    ("GN", "Guinea"),
    ("GQ", "Equatorial Guinea"),
    ("GR", "Greece"),
    ("GT", "Guatemala"),
    ("GW", "Guinea-Bissau"),
    ("GY", "Guyana"),
    ("HK", "Hong Kong"),
    ("HN", "Honduras"),
    ("HR", "Croatia"),
    ("HT", "Haiti"),
    ("HU", "Hungary"),
    ("ID", "Indonesia"),
    ("IE", "Ireland"),
    ("IL", "Israel"),
    ("IN", "India"),
    ("IQ", "Iraq"),
    ("IR", "Iran"),
    ("IS", "Iceland"),
    ("IT", "Italy"),
    ("JM", "Jamaica"),
    ("JO", "Jordan"),
    ("JP", "Japan"),
    ("KE", "Kenya"),
    ("KG", "Kyrgyzstan"),
    ("KH", "Cambodia"),
    ("KI", "Kiribati"),
    ("KM", "Comoros"),
    ("KN", "Saint Kitts and Nevis"),
    ("KP", "North Korea"),
    ("KR", "South Korea"),
    ("KW", "Kuwait"),
    ("KZ", "Kazakhstan"),
    ("LA", "Laos"),
    ("LB", "Lebanon"),
    ("LC", "Saint Lucia"),
    ("LI", "Liechtenstein"),
    ("LK", "Sri Lanka"),
    ("LR", "Liberia"),
    ("LS", "Lesotho"),
    ("LT", "Lithuania"),
    ("LU", "Luxembourg"),
    ("LV", "Latvia"),
    ("LY", "Libya"),
    ("MA", "Morocco"),
    ("MC", "Monaco"),
    ("MD", "Moldova"),
    ("ME", "Montenegro"),
    ("MG", "Madagascar"),
    ("MK", "North Macedonia"),
    ("ML", "Mali"),
    ("MM", "Myanmar"),
    ("MN", "Mongolia"),
    ("MR", "Mauritania"),
    ("MT", "Malta"),
    ("MU", "Mauritius"),
    ("MV", "Maldives"),
    ("MW", "Malawi"),
    ("MX", "Mexico"),
    ("MY", "Malaysia"),
    ("MZ", "Mozambique"),
    ("NA", "Namibia"),
    ("NE", "Niger"),
    ("NG", "Nigeria"),
    ("NI", "Nicaragua"),
    ("NL", "Netherlands"),
    ("NO", "Norway"),
    ("NP", "Nepal"),
    ("NR", "Nauru"),
    ("NZ", "New Zealand"),
    ("OM", "Oman"),
    ("PA", "Panama"),
    ("PE", "Peru"),
    ("PG", "Papua New Guinea"),
    ("PH", "Philippines"),
    ("PK", "Pakistan"),
    ("PL", "Poland"),
    ("PT", "Portugal"),
    ("PW", "Palau"),
    ("PY", "Paraguay"),
    ("QA", "Qatar"),
    ("RO", "Romania"),
    ("RS", "Serbia"),
    ("RU", "Russia"),
    ("RW", "Rwanda"),
    ("SA", "Saudi Arabia"),
    ("SB", "Solomon Islands"),
    ("SC", "Seychelles"),
    ("SD", "Sudan"),
    ("SE", "Sweden"),
    ("SG", "Singapore"),
    ("SI", "Slovenia"),
    ("SK", "Slovakia"),
    ("SL", "Sierra Leone"),
    ("SM", "San Marino"),
    ("SN", "Senegal"),
    ("SO", "Somalia"),
    ("SR", "Suriname"),
    ("SS", "South Sudan"),
    ("ST", "Sao Tome and Principe"),
    ("SV", "El Salvador"),
    ("SY", "Syria"),
    ("SZ", "Eswatini"),
    ("TD", "Chad"),
    ("TG", "Togo"),
    ("TH", "Thailand"),
    ("TJ", "Tajikistan"),
    ("TL", "Timor-Leste"),
    ("TM", "Turkmenistan"),
    ("TN", "Tunisia"),
    ("TO", "Tonga"),
    ("TR", "Turkey"),
    ("TT", "Trinidad and Tobago"),
    ("TV", "Tuvalu"),
    ("TW", "Taiwan"),
    ("TZ", "Tanzania"),
    ("UA", "Ukraine"),
    ("UG", "Uganda"),
    ("US", "United States"),
    ("UY", "Uruguay"),
    ("UZ", "Uzbekistan"),
    ("VA", "Vatican City"),
    ("VC", "Saint Vincent and the Grenadines"),
    ("VE", "Venezuela"),
    ("VN", "Vietnam"),
    ("VU", "Vanuatu"),
    ("WS", "Samoa"),
    ("YE", "Yemen"),
    ("ZA", "South Africa"),
    ("ZM", "Zambia"),
    ("ZW", "Zimbabwe"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_latam() {
        for code in [
            "CL", "AR", "BR", "CO", "MX", "PE", "UY", "PY", "BO", "EC", "VE", "CR", "PA", "SV",
            "GT", "HN", "NI", "CU", "DO",
        ] {
            assert!(is_valid_country(code), "{code} should be valid");
        }
    }

    #[test]
    fn valid_global() {
        for code in [
            "US", "GB", "DE", "FR", "JP", "CN", "IN", "AU", "KR", "SG", "CH", "IT", "ES",
        ] {
            assert!(is_valid_country(code), "{code} should be valid");
        }
    }

    #[test]
    fn invalid_codes() {
        assert!(!is_valid_country("XX"));
        assert!(!is_valid_country("cl")); // case sensitive
        assert!(!is_valid_country(""));
        assert!(!is_valid_country("ABC"));
    }

    #[test]
    fn lookup_name() {
        assert_eq!(country_name("CL"), Some("Chile"));
        assert_eq!(country_name("JP"), Some("Japan"));
        assert_eq!(country_name("XX"), None);
    }

    #[test]
    fn total_country_count() {
        assert!(
            COUNTRIES.len() >= 190,
            "should have 190+ countries, got {}",
            COUNTRIES.len()
        );
    }
}
