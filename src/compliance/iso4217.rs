//! ISO 4217 currency codes.

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
/// E.g., 150000 CLP (0 decimals) → "150000", 15050 USD (2 decimals) → "150.50"
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
        code: "JPY",
        name: "Japanese Yen",
        decimals: 0,
    },
    Currency {
        code: "KRW",
        name: "South Korean Won",
        decimals: 0,
    },
    Currency {
        code: "MXN",
        name: "Mexican Peso",
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
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_currencies() {
        assert!(is_valid_currency("CLP"));
        assert!(is_valid_currency("USD"));
        assert!(is_valid_currency("EUR"));
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

        let usd = get_currency("USD").unwrap();
        assert_eq!(usd.decimals, 2);
    }

    #[test]
    fn format_zero_decimals() {
        assert_eq!(format_amount(150000, "CLP"), "150000");
    }

    #[test]
    fn format_two_decimals() {
        assert_eq!(format_amount(15050, "USD"), "150.50");
    }

    #[test]
    fn format_unknown_currency() {
        assert_eq!(format_amount(100, "XXX"), "100");
    }
}
