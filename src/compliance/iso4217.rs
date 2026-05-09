//! ISO 4217 currency codes — expanded coverage.

use serde::{Deserialize, Serialize};

/// Currency metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Currency {
    pub code: &'static str,
    pub name: &'static str,
    pub decimals: u8,
}

/// Validate an ISO 4217 currency code.
pub fn is_valid_currency(code: &str) -> bool {
    CURRENCIES.iter().any(|c| c.code == code)
}

/// Lookup currency by code.
pub fn get_currency(code: &str) -> Option<&'static Currency> {
    CURRENCIES.iter().find(|c| c.code == code)
}

/// Format an integer amount using the currency's decimal places.
pub fn format_amount(amount: u64, code: &str) -> String {
    match get_currency(code) {
        Some(c) if c.decimals > 0 => {
            let divisor = 10u64.pow(c.decimals as u32);
            let whole = amount / divisor;
            let frac = amount % divisor;
            format!("{whole}.{frac:0>width$}", width = c.decimals as usize)
        }
        _ => amount.to_string(),
    }
}

const CURRENCIES: &[Currency] = &[
    // Americas
    Currency {
        code: "ARS",
        name: "Argentine Peso",
        decimals: 2,
    },
    Currency {
        code: "BOB",
        name: "Boliviano",
        decimals: 2,
    },
    Currency {
        code: "BRL",
        name: "Brazilian Real",
        decimals: 2,
    },
    Currency {
        code: "CAD",
        name: "Canadian Dollar",
        decimals: 2,
    },
    Currency {
        code: "CLP",
        name: "Chilean Peso",
        decimals: 0,
    },
    Currency {
        code: "COP",
        name: "Colombian Peso",
        decimals: 2,
    },
    Currency {
        code: "CRC",
        name: "Costa Rican Colon",
        decimals: 2,
    },
    Currency {
        code: "CUP",
        name: "Cuban Peso",
        decimals: 2,
    },
    Currency {
        code: "DOP",
        name: "Dominican Peso",
        decimals: 2,
    },
    Currency {
        code: "GTQ",
        name: "Guatemalan Quetzal",
        decimals: 2,
    },
    Currency {
        code: "HNL",
        name: "Honduran Lempira",
        decimals: 2,
    },
    Currency {
        code: "MXN",
        name: "Mexican Peso",
        decimals: 2,
    },
    Currency {
        code: "NIO",
        name: "Nicaraguan Cordoba",
        decimals: 2,
    },
    Currency {
        code: "PAB",
        name: "Panamanian Balboa",
        decimals: 2,
    },
    Currency {
        code: "PEN",
        name: "Peruvian Sol",
        decimals: 2,
    },
    Currency {
        code: "PYG",
        name: "Paraguayan Guarani",
        decimals: 0,
    },
    Currency {
        code: "USD",
        name: "US Dollar",
        decimals: 2,
    },
    Currency {
        code: "UYU",
        name: "Uruguayan Peso",
        decimals: 2,
    },
    Currency {
        code: "VES",
        name: "Venezuelan Bolivar",
        decimals: 2,
    },
    // Europe
    Currency {
        code: "EUR",
        name: "Euro",
        decimals: 2,
    },
    Currency {
        code: "GBP",
        name: "British Pound",
        decimals: 2,
    },
    Currency {
        code: "CHF",
        name: "Swiss Franc",
        decimals: 2,
    },
    Currency {
        code: "SEK",
        name: "Swedish Krona",
        decimals: 2,
    },
    Currency {
        code: "NOK",
        name: "Norwegian Krone",
        decimals: 2,
    },
    Currency {
        code: "DKK",
        name: "Danish Krone",
        decimals: 2,
    },
    Currency {
        code: "PLN",
        name: "Polish Zloty",
        decimals: 2,
    },
    Currency {
        code: "CZK",
        name: "Czech Koruna",
        decimals: 2,
    },
    Currency {
        code: "HUF",
        name: "Hungarian Forint",
        decimals: 2,
    },
    Currency {
        code: "RON",
        name: "Romanian Leu",
        decimals: 2,
    },
    Currency {
        code: "BGN",
        name: "Bulgarian Lev",
        decimals: 2,
    },
    Currency {
        code: "HRK",
        name: "Croatian Kuna",
        decimals: 2,
    },
    Currency {
        code: "RUB",
        name: "Russian Ruble",
        decimals: 2,
    },
    Currency {
        code: "TRY",
        name: "Turkish Lira",
        decimals: 2,
    },
    Currency {
        code: "UAH",
        name: "Ukrainian Hryvnia",
        decimals: 2,
    },
    // Asia-Pacific
    Currency {
        code: "JPY",
        name: "Japanese Yen",
        decimals: 0,
    },
    Currency {
        code: "CNY",
        name: "Chinese Yuan",
        decimals: 2,
    },
    Currency {
        code: "KRW",
        name: "South Korean Won",
        decimals: 0,
    },
    Currency {
        code: "INR",
        name: "Indian Rupee",
        decimals: 2,
    },
    Currency {
        code: "IDR",
        name: "Indonesian Rupiah",
        decimals: 2,
    },
    Currency {
        code: "THB",
        name: "Thai Baht",
        decimals: 2,
    },
    Currency {
        code: "VND",
        name: "Vietnamese Dong",
        decimals: 0,
    },
    Currency {
        code: "PHP",
        name: "Philippine Peso",
        decimals: 2,
    },
    Currency {
        code: "MYR",
        name: "Malaysian Ringgit",
        decimals: 2,
    },
    Currency {
        code: "SGD",
        name: "Singapore Dollar",
        decimals: 2,
    },
    Currency {
        code: "HKD",
        name: "Hong Kong Dollar",
        decimals: 2,
    },
    Currency {
        code: "TWD",
        name: "Taiwan Dollar",
        decimals: 2,
    },
    Currency {
        code: "AUD",
        name: "Australian Dollar",
        decimals: 2,
    },
    Currency {
        code: "NZD",
        name: "New Zealand Dollar",
        decimals: 2,
    },
    // Middle East & Africa
    Currency {
        code: "AED",
        name: "UAE Dirham",
        decimals: 2,
    },
    Currency {
        code: "SAR",
        name: "Saudi Riyal",
        decimals: 2,
    },
    Currency {
        code: "QAR",
        name: "Qatari Riyal",
        decimals: 2,
    },
    Currency {
        code: "KWD",
        name: "Kuwaiti Dinar",
        decimals: 3,
    },
    Currency {
        code: "BHD",
        name: "Bahraini Dinar",
        decimals: 3,
    },
    Currency {
        code: "OMR",
        name: "Omani Rial",
        decimals: 3,
    },
    Currency {
        code: "ILS",
        name: "Israeli Shekel",
        decimals: 2,
    },
    Currency {
        code: "EGP",
        name: "Egyptian Pound",
        decimals: 2,
    },
    Currency {
        code: "ZAR",
        name: "South African Rand",
        decimals: 2,
    },
    Currency {
        code: "NGN",
        name: "Nigerian Naira",
        decimals: 2,
    },
    Currency {
        code: "KES",
        name: "Kenyan Shilling",
        decimals: 2,
    },
    Currency {
        code: "GHS",
        name: "Ghanaian Cedi",
        decimals: 2,
    },
    Currency {
        code: "MAD",
        name: "Moroccan Dirham",
        decimals: 2,
    },
    Currency {
        code: "TND",
        name: "Tunisian Dinar",
        decimals: 3,
    },
    // Supranational
    Currency {
        code: "XAU",
        name: "Gold (troy oz)",
        decimals: 0,
    },
    Currency {
        code: "XAG",
        name: "Silver (troy oz)",
        decimals: 0,
    },
    Currency {
        code: "XDR",
        name: "SDR (IMF)",
        decimals: 0,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_latam_currencies() {
        for code in [
            "CLP", "ARS", "BRL", "COP", "MXN", "PEN", "UYU", "PYG", "BOB", "VES", "CRC", "DOP",
        ] {
            assert!(is_valid_currency(code), "{code} should be valid");
        }
    }

    #[test]
    fn valid_global_currencies() {
        for code in [
            "USD", "EUR", "GBP", "JPY", "CNY", "CHF", "AUD", "SGD", "KRW",
        ] {
            assert!(is_valid_currency(code), "{code} should be valid");
        }
    }

    #[test]
    fn invalid_currencies() {
        assert!(!is_valid_currency("XXX"));
        assert!(!is_valid_currency(""));
    }

    #[test]
    fn lookup_currency() {
        let clp = get_currency("CLP").unwrap();
        assert_eq!(clp.name, "Chilean Peso");
        assert_eq!(clp.decimals, 0);

        let kwd = get_currency("KWD").unwrap();
        assert_eq!(kwd.decimals, 3);
    }

    #[test]
    fn format_zero_decimals() {
        assert_eq!(format_amount(150000, "CLP"), "150000");
        assert_eq!(format_amount(150000, "JPY"), "150000");
    }

    #[test]
    fn format_two_decimals() {
        assert_eq!(format_amount(15050, "USD"), "150.50");
    }

    #[test]
    fn format_three_decimals() {
        assert_eq!(format_amount(1500, "KWD"), "1.500");
    }

    #[test]
    fn format_unknown_currency() {
        assert_eq!(format_amount(100, "XXX"), "100");
    }

    #[test]
    fn total_currency_count() {
        assert!(
            CURRENCIES.len() >= 60,
            "should have 60+ currencies, got {}",
            CURRENCIES.len()
        );
    }
}
