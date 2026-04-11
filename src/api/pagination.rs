//! Pagination utilities for API endpoints.

use serde::{Deserialize, Serialize};

/// Maximum allowed page size.
const MAX_LIMIT: usize = 100;
/// Default page size when not specified.
const DEFAULT_LIMIT: usize = 20;

/// Query parameters for paginated endpoints.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PaginationParams {
    pub page: Option<usize>,
    pub limit: Option<usize>,
    #[allow(dead_code)]
    pub cursor: Option<String>,
}

impl PaginationParams {
    /// Effective page number (1-based, defaults to 1).
    pub fn page(&self) -> usize {
        self.page.unwrap_or(1).max(1)
    }

    /// Effective page size, capped at `MAX_LIMIT`.
    pub fn limit(&self) -> usize {
        self.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT)
    }

    /// Offset for database/store queries (0-based).
    pub fn offset(&self) -> usize {
        (self.page() - 1) * self.limit()
    }
}

/// Metadata about a paginated result set.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PaginationMeta {
    pub total: usize,
    pub page: usize,
    pub limit: usize,
    pub total_pages: usize,
    pub has_next: bool,
    pub next_cursor: Option<String>,
}

/// A paginated response wrapping a data slice and its metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T: Serialize> {
    pub data: Vec<T>,
    pub pagination: PaginationMeta,
}

impl<T: Serialize> PaginatedResponse<T> {
    /// Build a paginated response from a data slice and total count.
    pub fn new(data: Vec<T>, total: usize, params: &PaginationParams) -> Self {
        let page = params.page();
        let limit = params.limit();
        let total_pages = if total == 0 { 1 } else { total.div_ceil(limit) };
        let has_next = page < total_pages;

        Self {
            data,
            pagination: PaginationMeta {
                total,
                page,
                limit,
                total_pages,
                has_next,
                next_cursor: None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults() {
        let p = PaginationParams::default();
        assert_eq!(p.page(), 1);
        assert_eq!(p.limit(), DEFAULT_LIMIT);
        assert_eq!(p.offset(), 0);
    }

    #[test]
    fn parse_page_and_limit() {
        let json = r#"{"page":2,"limit":10}"#;
        let p: PaginationParams = serde_json::from_str(json).unwrap();
        assert_eq!(p.page(), 2);
        assert_eq!(p.limit(), 10);
        assert_eq!(p.offset(), 10);
    }

    #[test]
    fn limit_capped_at_max() {
        let p = PaginationParams {
            page: Some(1),
            limit: Some(500),
            cursor: None,
        };
        assert_eq!(p.limit(), MAX_LIMIT);
    }

    #[test]
    fn limit_zero_becomes_one() {
        let p = PaginationParams {
            page: Some(1),
            limit: Some(0),
            cursor: None,
        };
        assert_eq!(p.limit(), 1);
    }

    #[test]
    fn page_zero_becomes_one() {
        let p = PaginationParams {
            page: Some(0),
            limit: None,
            cursor: None,
        };
        assert_eq!(p.page(), 1);
        assert_eq!(p.offset(), 0);
    }

    #[test]
    fn paginated_response_metadata() {
        let params = PaginationParams {
            page: Some(3),
            limit: Some(10),
            cursor: None,
        };
        let resp = PaginatedResponse::new(vec![1, 2, 3], 50, &params);

        assert_eq!(resp.pagination.total, 50);
        assert_eq!(resp.pagination.page, 3);
        assert_eq!(resp.pagination.limit, 10);
        assert_eq!(resp.pagination.total_pages, 5);
        assert!(resp.pagination.has_next);
    }

    #[test]
    fn paginated_response_last_page() {
        let params = PaginationParams {
            page: Some(5),
            limit: Some(10),
            cursor: None,
        };
        let resp = PaginatedResponse::new(vec![41, 42, 43], 43, &params);

        assert_eq!(resp.pagination.total_pages, 5);
        assert!(!resp.pagination.has_next);
    }

    #[test]
    fn paginated_response_serde_roundtrip() {
        let params = PaginationParams {
            page: Some(1),
            limit: Some(5),
            cursor: None,
        };
        let resp = PaginatedResponse::new(vec!["a", "b"], 2, &params);

        let json = serde_json::to_string(&resp).unwrap();
        let decoded: PaginatedResponse<String> = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.pagination.total, 2);
        assert_eq!(decoded.data.len(), 2);
    }
}
