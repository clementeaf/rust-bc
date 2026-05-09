//! Compliance validation endpoints:
//!   POST /api/v1/compliance/validate/pacs008   — validate pacs.008 message
//!   POST /api/v1/compliance/validate/pacs002   — validate pacs.002 message
//!   POST /api/v1/compliance/validate/pacs004   — validate pacs.004 message
//!   POST /api/v1/compliance/validate/pain001   — validate pain.001 message
//!   POST /api/v1/compliance/validate/pain002   — validate pain.002 message
//!   POST /api/v1/compliance/validate/camt053   — validate camt.053 message
//!   POST /api/v1/compliance/validate/camt052   — validate camt.052 message
//!   GET  /api/v1/compliance/countries           — list all ISO 3166 countries
//!   GET  /api/v1/compliance/currencies          — list all ISO 4217 currencies

use actix_web::{get, post, web, HttpResponse};

use crate::api::errors::{ApiResponse, ApiResult};
use crate::compliance::{iso20022, iso3166, iso4217};

/// POST /api/v1/compliance/validate/pacs008
#[post("/compliance/validate/pacs008")]
pub async fn validate_pacs008(body: web::Json<iso20022::Pacs008>) -> ApiResult<HttpResponse> {
    let trace = uuid::Uuid::new_v4().to_string();
    match iso20022::validate_pacs008(&body) {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::success(
            serde_json::json!({"valid": true, "message_type": "pacs.008"}),
            trace,
        ))),
        Err(e) => Ok(HttpResponse::BadRequest().json(ApiResponse::<()>::error(
            crate::api::errors::ErrorDto {
                code: "COMPLIANCE_ERROR".into(),
                message: e.to_string(),
                field: None,
            },
            400,
        ))),
    }
}

/// POST /api/v1/compliance/validate/pacs002
#[post("/compliance/validate/pacs002")]
pub async fn validate_pacs002(body: web::Json<iso20022::Pacs002>) -> ApiResult<HttpResponse> {
    let trace = uuid::Uuid::new_v4().to_string();
    match iso20022::validate_pacs002(&body) {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::success(
            serde_json::json!({"valid": true, "message_type": "pacs.002"}),
            trace,
        ))),
        Err(e) => Ok(HttpResponse::BadRequest().json(ApiResponse::<()>::error(
            crate::api::errors::ErrorDto {
                code: "COMPLIANCE_ERROR".into(),
                message: e.to_string(),
                field: None,
            },
            400,
        ))),
    }
}

/// POST /api/v1/compliance/validate/pacs004
#[post("/compliance/validate/pacs004")]
pub async fn validate_pacs004(body: web::Json<iso20022::Pacs004>) -> ApiResult<HttpResponse> {
    let trace = uuid::Uuid::new_v4().to_string();
    match iso20022::validate_pacs004(&body) {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::success(
            serde_json::json!({"valid": true, "message_type": "pacs.004"}),
            trace,
        ))),
        Err(e) => Ok(HttpResponse::BadRequest().json(ApiResponse::<()>::error(
            crate::api::errors::ErrorDto {
                code: "COMPLIANCE_ERROR".into(),
                message: e.to_string(),
                field: None,
            },
            400,
        ))),
    }
}

/// POST /api/v1/compliance/validate/pain001
#[post("/compliance/validate/pain001")]
pub async fn validate_pain001(body: web::Json<iso20022::Pain001>) -> ApiResult<HttpResponse> {
    let trace = uuid::Uuid::new_v4().to_string();
    match iso20022::validate_pain001(&body) {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::success(
            serde_json::json!({"valid": true, "message_type": "pain.001"}),
            trace,
        ))),
        Err(e) => Ok(HttpResponse::BadRequest().json(ApiResponse::<()>::error(
            crate::api::errors::ErrorDto {
                code: "COMPLIANCE_ERROR".into(),
                message: e.to_string(),
                field: None,
            },
            400,
        ))),
    }
}

/// POST /api/v1/compliance/validate/pain002
#[post("/compliance/validate/pain002")]
pub async fn validate_pain002(body: web::Json<iso20022::Pain002>) -> ApiResult<HttpResponse> {
    let trace = uuid::Uuid::new_v4().to_string();
    match iso20022::validate_pain002(&body) {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::success(
            serde_json::json!({"valid": true, "message_type": "pain.002"}),
            trace,
        ))),
        Err(e) => Ok(HttpResponse::BadRequest().json(ApiResponse::<()>::error(
            crate::api::errors::ErrorDto {
                code: "COMPLIANCE_ERROR".into(),
                message: e.to_string(),
                field: None,
            },
            400,
        ))),
    }
}

/// POST /api/v1/compliance/validate/camt053
#[post("/compliance/validate/camt053")]
pub async fn validate_camt053(body: web::Json<iso20022::Camt053>) -> ApiResult<HttpResponse> {
    let trace = uuid::Uuid::new_v4().to_string();
    match iso20022::validate_camt053(&body) {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::success(
            serde_json::json!({"valid": true, "message_type": "camt.053"}),
            trace,
        ))),
        Err(e) => Ok(HttpResponse::BadRequest().json(ApiResponse::<()>::error(
            crate::api::errors::ErrorDto {
                code: "COMPLIANCE_ERROR".into(),
                message: e.to_string(),
                field: None,
            },
            400,
        ))),
    }
}

/// POST /api/v1/compliance/validate/camt052
#[post("/compliance/validate/camt052")]
pub async fn validate_camt052(body: web::Json<iso20022::Camt052>) -> ApiResult<HttpResponse> {
    let trace = uuid::Uuid::new_v4().to_string();
    match iso20022::validate_camt052(&body) {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::success(
            serde_json::json!({"valid": true, "message_type": "camt.052"}),
            trace,
        ))),
        Err(e) => Ok(HttpResponse::BadRequest().json(ApiResponse::<()>::error(
            crate::api::errors::ErrorDto {
                code: "COMPLIANCE_ERROR".into(),
                message: e.to_string(),
                field: None,
            },
            400,
        ))),
    }
}

/// GET /api/v1/compliance/countries
#[get("/compliance/countries")]
pub async fn list_countries() -> ApiResult<HttpResponse> {
    let trace = uuid::Uuid::new_v4().to_string();
    let countries: Vec<serde_json::Value> = [
        "AD", "AE", "AF", "AG", "AL", "AM", "AO", "AR", "AT", "AU", "AZ", "BA", "BB", "BD", "BE",
        "BF", "BG", "BH", "BI", "BJ", "BN", "BO", "BR", "BS", "BT", "BW", "BY", "BZ", "CA", "CD",
        "CF", "CG", "CH", "CI", "CL", "CM", "CN", "CO", "CR", "CU", "CV", "CY", "CZ", "DE", "DJ",
        "DK", "DM", "DO", "DZ", "EC", "EE", "EG", "ER", "ES", "ET", "FI", "FJ", "FR", "GA", "GB",
        "GD", "GE", "GH", "GM", "GN", "GQ", "GR", "GT", "GW", "GY", "HK", "HN", "HR", "HT", "HU",
        "ID", "IE", "IL", "IN", "IQ", "IR", "IS", "IT", "JM", "JO", "JP", "KE", "KG", "KH", "KI",
        "KM", "KN", "KP", "KR", "KW", "KZ", "LA", "LB", "LC", "LI", "LK", "LR", "LS", "LT", "LU",
        "LV", "LY", "MA", "MC", "MD", "ME", "MG", "MK", "ML", "MM", "MN", "MR", "MT", "MU", "MV",
        "MW", "MX", "MY", "MZ", "NA", "NE", "NG", "NI", "NL", "NO", "NP", "NR", "NZ", "OM", "PA",
        "PE", "PG", "PH", "PK", "PL", "PT", "PW", "PY", "QA", "RO", "RS", "RU", "RW", "SA", "SB",
        "SC", "SD", "SE", "SG", "SI", "SK", "SL", "SM", "SN", "SO", "SR", "SS", "ST", "SV", "SY",
        "SZ", "TD", "TG", "TH", "TJ", "TL", "TM", "TN", "TO", "TR", "TT", "TV", "TW", "TZ", "UA",
        "UG", "US", "UY", "UZ", "VA", "VC", "VE", "VN", "VU", "WS", "YE", "ZA", "ZM", "ZW",
    ]
    .iter()
    .filter_map(|code| {
        iso3166::country_name(code).map(|name| serde_json::json!({"code": code, "name": name}))
    })
    .collect();

    Ok(HttpResponse::Ok().json(ApiResponse::success(countries, trace)))
}

/// GET /api/v1/compliance/currencies
#[get("/compliance/currencies")]
pub async fn list_currencies() -> ApiResult<HttpResponse> {
    let trace = uuid::Uuid::new_v4().to_string();
    let currencies: Vec<serde_json::Value> = [
        "ARS", "BOB", "BRL", "CAD", "CLP", "COP", "CRC", "CUP", "DOP", "GTQ", "HNL", "MXN", "NIO",
        "PAB", "PEN", "PYG", "USD", "UYU", "VES", "EUR", "GBP", "CHF", "SEK", "NOK", "DKK", "PLN",
        "CZK", "HUF", "RON", "BGN", "HRK", "RUB", "TRY", "UAH", "JPY", "CNY", "KRW", "INR", "IDR",
        "THB", "VND", "PHP", "MYR", "SGD", "HKD", "TWD", "AUD", "NZD", "AED", "SAR", "QAR", "KWD",
        "BHD", "OMR", "ILS", "EGP", "ZAR", "NGN", "KES", "GHS", "MAD", "TND", "XAU", "XAG", "XDR",
    ]
    .iter()
    .filter_map(|code| {
        iso4217::get_currency(code)
            .map(|c| serde_json::json!({"code": c.code, "name": c.name, "decimals": c.decimals}))
    })
    .collect();

    Ok(HttpResponse::Ok().json(ApiResponse::success(currencies, trace)))
}
