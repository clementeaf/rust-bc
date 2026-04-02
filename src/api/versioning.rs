use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// API version semantic format
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct ApiVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl ApiVersion {
    /// Create a new API version
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }

    /// Parse version from string (e.g., "1.0.0")
    pub fn parse(version_str: &str) -> Result<Self, String> {
        let parts: Vec<&str> = version_str.split('.').collect();
        if parts.len() != 3 {
            return Err(format!(
                "Invalid version format: expected 'major.minor.patch', got '{}'",
                version_str
            ));
        }

        let major = parts[0]
            .parse::<u32>()
            .map_err(|_| format!("Invalid major version: {}", parts[0]))?;
        let minor = parts[1]
            .parse::<u32>()
            .map_err(|_| format!("Invalid minor version: {}", parts[1]))?;
        let patch = parts[2]
            .parse::<u32>()
            .map_err(|_| format!("Invalid patch version: {}", parts[2]))?;

        Ok(Self { major, minor, patch })
    }

    /// Check if this version is compatible with a minimum required version
    pub fn is_compatible_with(&self, minimum: ApiVersion) -> bool {
        if self.major != minimum.major {
            return self.major > minimum.major;
        }
        if self.minor != minimum.minor {
            return self.minor > minimum.minor;
        }
        self.patch >= minimum.patch
    }

    /// Check if this version supports a feature
    pub fn supports_feature(&self, feature_introduced: ApiVersion) -> bool {
        self.is_compatible_with(feature_introduced)
    }
}

impl PartialOrd for ApiVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ApiVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.major.cmp(&other.major) {
            Ordering::Equal => match self.minor.cmp(&other.minor) {
                Ordering::Equal => self.patch.cmp(&other.patch),
                other => other,
            },
            other => other,
        }
    }
}

impl std::fmt::Display for ApiVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// API feature version mapping
#[derive(Clone, Debug)]
pub struct ApiFeatureMatrix {
    /// Minimum supported API version
    pub min_version: ApiVersion,
    /// Current API version
    pub current_version: ApiVersion,
    /// Feature availability per version
    features: std::collections::HashMap<String, ApiVersion>,
}

impl ApiFeatureMatrix {
    /// Create a new feature matrix
    pub fn new(min_version: ApiVersion, current_version: ApiVersion) -> Self {
        Self {
            min_version,
            current_version,
            features: std::collections::HashMap::new(),
        }
    }

    /// Register a feature as available from a specific version
    pub fn add_feature(mut self, feature_name: &str, introduced_version: ApiVersion) -> Self {
        self.features
            .insert(feature_name.to_string(), introduced_version);
        self
    }

    /// Check if a feature is available in a given version
    pub fn is_feature_available(&self, feature: &str, version: ApiVersion) -> bool {
        self.features
            .get(feature)
            .map(|&introduced| version.supports_feature(introduced))
            .unwrap_or(false)
    }

    /// Get all features available in a version
    pub fn get_available_features(&self, version: ApiVersion) -> Vec<String> {
        self.features
            .iter()
            .filter(|(_, &introduced)| version.supports_feature(introduced))
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Validate version is within supported range
    pub fn is_version_supported(&self, version: ApiVersion) -> bool {
        version >= self.min_version && version <= self.current_version
    }
}

/// API version negotiation result
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VersionNegotiation {
    /// Requested version
    pub requested: Option<ApiVersion>,
    /// Negotiated version to use
    pub negotiated: ApiVersion,
    /// Whether the requested version was honored
    pub exact_match: bool,
}

impl VersionNegotiation {
    /// Negotiate API version
    pub fn negotiate(
        requested: Option<&str>,
        current: ApiVersion,
        minimum: ApiVersion,
    ) -> Result<Self, String> {
        match requested {
            Some(req_str) => {
                let requested_version = ApiVersion::parse(req_str)?;

                if !requested_version.is_compatible_with(minimum)
                    || requested_version > current
                {
                    return Err(format!(
                        "Requested version {} not supported. Supported range: {}-{}",
                        requested_version, minimum, current
                    ));
                }

                Ok(Self {
                    requested: Some(requested_version),
                    negotiated: requested_version,
                    exact_match: true,
                })
            }
            None => {
                Ok(Self {
                    requested: None,
                    negotiated: current,
                    exact_match: false,
                })
            }
        }
    }

    /// Negotiate with fallback to latest compatible
    pub fn negotiate_with_fallback(
        requested: Option<&str>,
        current: ApiVersion,
        minimum: ApiVersion,
    ) -> Self {
        match requested {
            Some(req_str) => {
                match ApiVersion::parse(req_str) {
                    Ok(requested_version) => {
                        if requested_version.is_compatible_with(minimum)
                            && requested_version <= current
                        {
                            Self {
                                requested: Some(requested_version),
                                negotiated: requested_version,
                                exact_match: true,
                            }
                        } else {
                            Self {
                                requested: Some(requested_version),
                                negotiated: current,
                                exact_match: false,
                            }
                        }
                    }
                    Err(_) => Self {
                        requested: None,
                        negotiated: current,
                        exact_match: false,
                    },
                }
            }
            None => Self {
                requested: None,
                negotiated: current,
                exact_match: false,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_version_parse_valid() {
        let v = ApiVersion::parse("1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
    }

    #[test]
    fn test_api_version_parse_invalid_format() {
        assert!(ApiVersion::parse("1.2").is_err());
        assert!(ApiVersion::parse("1.2.3.4").is_err());
    }

    #[test]
    fn test_api_version_parse_invalid_numbers() {
        assert!(ApiVersion::parse("a.2.3").is_err());
        assert!(ApiVersion::parse("1.b.3").is_err());
        assert!(ApiVersion::parse("1.2.c").is_err());
    }

    #[test]
    fn test_api_version_to_string() {
        let v = ApiVersion::new(1, 2, 3);
        assert_eq!(v.to_string(), "1.2.3");
    }

    #[test]
    fn test_api_version_compatibility_same() {
        let v1 = ApiVersion::new(1, 2, 3);
        let v2 = ApiVersion::new(1, 2, 3);
        assert!(v1.is_compatible_with(v2));
    }

    #[test]
    fn test_api_version_compatibility_newer() {
        let v1 = ApiVersion::new(1, 2, 4);
        let v2 = ApiVersion::new(1, 2, 3);
        assert!(v1.is_compatible_with(v2));
    }

    #[test]
    fn test_api_version_compatibility_older() {
        let v1 = ApiVersion::new(1, 2, 2);
        let v2 = ApiVersion::new(1, 2, 3);
        assert!(!v1.is_compatible_with(v2));
    }

    #[test]
    fn test_api_version_compatibility_minor_version() {
        let v1 = ApiVersion::new(1, 3, 0);
        let v2 = ApiVersion::new(1, 2, 5);
        assert!(v1.is_compatible_with(v2));
    }

    #[test]
    fn test_api_version_compatibility_major_version() {
        let v1 = ApiVersion::new(2, 0, 0);
        let v2 = ApiVersion::new(1, 9, 9);
        assert!(v1.is_compatible_with(v2));
    }

    #[test]
    fn test_api_version_major_version_break() {
        let v1 = ApiVersion::new(1, 5, 0);
        let v2 = ApiVersion::new(2, 0, 0);
        assert!(!v1.is_compatible_with(v2));
    }

    #[test]
    fn test_api_version_ordering() {
        let v1 = ApiVersion::new(1, 2, 3);
        let v2 = ApiVersion::new(1, 2, 4);
        let v3 = ApiVersion::new(1, 3, 0);
        let v4 = ApiVersion::new(2, 0, 0);

        assert!(v1 < v2);
        assert!(v2 < v3);
        assert!(v3 < v4);
        assert!(v4 > v1);
    }

    #[test]
    fn test_feature_matrix_creation() {
        let min = ApiVersion::new(1, 0, 0);
        let current = ApiVersion::new(1, 5, 0);
        let matrix = ApiFeatureMatrix::new(min, current);

        assert_eq!(matrix.min_version, min);
        assert_eq!(matrix.current_version, current);
    }

    #[test]
    fn test_feature_matrix_add_feature() {
        let matrix = ApiFeatureMatrix::new(ApiVersion::new(1, 0, 0), ApiVersion::new(1, 5, 0))
            .add_feature("consensus-v2", ApiVersion::new(1, 2, 0))
            .add_feature("batch-operations", ApiVersion::new(1, 3, 0));

        assert!(matrix.is_feature_available("consensus-v2", ApiVersion::new(1, 2, 0)));
        assert!(matrix.is_feature_available("batch-operations", ApiVersion::new(1, 3, 5)));
        assert!(!matrix.is_feature_available("consensus-v2", ApiVersion::new(1, 1, 9)));
    }

    #[test]
    fn test_feature_matrix_get_available_features() {
        let matrix = ApiFeatureMatrix::new(ApiVersion::new(1, 0, 0), ApiVersion::new(1, 5, 0))
            .add_feature("feature-a", ApiVersion::new(1, 0, 0))
            .add_feature("feature-b", ApiVersion::new(1, 2, 0))
            .add_feature("feature-c", ApiVersion::new(1, 4, 0));

        let features_v1 = matrix.get_available_features(ApiVersion::new(1, 0, 0));
        assert_eq!(features_v1.len(), 1);
        assert!(features_v1.contains(&"feature-a".to_string()));

        let features_v13 = matrix.get_available_features(ApiVersion::new(1, 3, 0));
        assert_eq!(features_v13.len(), 2);

        let features_v15 = matrix.get_available_features(ApiVersion::new(1, 5, 0));
        assert_eq!(features_v15.len(), 3);
    }

    #[test]
    fn test_feature_matrix_is_version_supported() {
        let matrix =
            ApiFeatureMatrix::new(ApiVersion::new(1, 2, 0), ApiVersion::new(1, 5, 0));

        assert!(!matrix.is_version_supported(ApiVersion::new(1, 1, 9)));
        assert!(matrix.is_version_supported(ApiVersion::new(1, 2, 0)));
        assert!(matrix.is_version_supported(ApiVersion::new(1, 3, 5)));
        assert!(matrix.is_version_supported(ApiVersion::new(1, 5, 0)));
        assert!(!matrix.is_version_supported(ApiVersion::new(1, 5, 1)));
    }

    #[test]
    fn test_version_negotiation_with_exact_match() {
        let current = ApiVersion::new(1, 5, 0);
        let minimum = ApiVersion::new(1, 0, 0);

        let result = VersionNegotiation::negotiate(Some("1.3.0"), current, minimum).unwrap();

        assert!(result.requested.is_some());
        assert_eq!(result.negotiated, ApiVersion::new(1, 3, 0));
        assert!(result.exact_match);
    }

    #[test]
    fn test_version_negotiation_no_request() {
        let current = ApiVersion::new(1, 5, 0);
        let minimum = ApiVersion::new(1, 0, 0);

        let result = VersionNegotiation::negotiate(None, current, minimum).unwrap();

        assert!(result.requested.is_none());
        assert_eq!(result.negotiated, current);
        assert!(!result.exact_match);
    }

    #[test]
    fn test_version_negotiation_unsupported() {
        let current = ApiVersion::new(1, 5, 0);
        let minimum = ApiVersion::new(1, 2, 0);

        assert!(VersionNegotiation::negotiate(Some("1.1.0"), current, minimum).is_err());
        assert!(VersionNegotiation::negotiate(Some("1.6.0"), current, minimum).is_err());
    }

    #[test]
    fn test_version_negotiation_with_fallback() {
        let current = ApiVersion::new(1, 5, 0);
        let minimum = ApiVersion::new(1, 0, 0);

        let result = VersionNegotiation::negotiate_with_fallback(
            Some("1.3.0"),
            current,
            minimum,
        );
        assert_eq!(result.negotiated, ApiVersion::new(1, 3, 0));
        assert!(result.exact_match);
    }

    #[test]
    fn test_version_negotiation_fallback_to_current() {
        let current = ApiVersion::new(1, 5, 0);
        let minimum = ApiVersion::new(1, 0, 0);

        let result = VersionNegotiation::negotiate_with_fallback(
            Some("1.6.0"),
            current,
            minimum,
        );
        assert_eq!(result.negotiated, current);
        assert!(!result.exact_match);
    }

    #[test]
    fn test_version_negotiation_fallback_invalid_format() {
        let current = ApiVersion::new(1, 5, 0);
        let minimum = ApiVersion::new(1, 0, 0);

        let result = VersionNegotiation::negotiate_with_fallback(
            Some("invalid"),
            current,
            minimum,
        );
        assert_eq!(result.negotiated, current);
        assert!(!result.exact_match);
    }
}
