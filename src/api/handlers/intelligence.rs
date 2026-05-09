//! Intelligence layer endpoints:
//!   POST /api/v1/intelligence/anomaly    — check a data point for anomalies
//!   POST /api/v1/intelligence/risk       — evaluate risk score
//!   POST /api/v1/intelligence/patterns   — analyze transactions for patterns

use actix_web::{post, web, HttpResponse};
use serde::Deserialize;

use crate::api::errors::{ApiResponse, ApiResult};
use crate::intelligence::anomaly::{AnomalyConfig, AnomalyDetector, DataPoint};
use crate::intelligence::patterns::{PatternEngine, TxRecord};
use crate::intelligence::risk::{RiskEngine, RiskInput};

/// POST /api/v1/intelligence/anomaly — feed data points, detect anomalies
#[derive(Deserialize)]
pub struct AnomalyRequest {
    pub points: Vec<DataPoint>,
    pub z_threshold: Option<f64>,
}

#[post("/intelligence/anomaly")]
pub async fn detect_anomalies(body: web::Json<AnomalyRequest>) -> ApiResult<HttpResponse> {
    let trace = uuid::Uuid::new_v4().to_string();

    let config = AnomalyConfig {
        z_threshold: body.z_threshold.unwrap_or(3.0),
        min_samples: 10,
        window_size: 1000,
    };
    let mut detector = AnomalyDetector::new(config);

    for point in &body.points {
        detector.observe(point);
    }

    let stats = detector.stats();
    let anomalies = detector.anomalies().to_vec();

    Ok(HttpResponse::Ok().json(ApiResponse::success(
        serde_json::json!({
            "anomalies": anomalies,
            "stats": stats,
            "points_analyzed": body.points.len(),
        }),
        trace,
    )))
}

/// POST /api/v1/intelligence/risk — evaluate risk for a transaction
#[post("/intelligence/risk")]
pub async fn evaluate_risk(body: web::Json<RiskInput>) -> ApiResult<HttpResponse> {
    let trace = uuid::Uuid::new_v4().to_string();
    let engine = RiskEngine::with_defaults();
    let result = engine.evaluate(&body);
    Ok(HttpResponse::Ok().json(ApiResponse::success(result, trace)))
}

/// POST /api/v1/intelligence/patterns — analyze transactions for patterns
#[derive(Deserialize)]
pub struct PatternRequest {
    pub transactions: Vec<TxRecord>,
    pub velocity_threshold: Option<usize>,
    pub structuring_threshold: Option<u64>,
}

#[post("/intelligence/patterns")]
pub async fn detect_patterns(body: web::Json<PatternRequest>) -> ApiResult<HttpResponse> {
    let trace = uuid::Uuid::new_v4().to_string();

    let mut engine = PatternEngine::new();
    if let Some(v) = body.velocity_threshold {
        engine.velocity_threshold = v;
    }
    if let Some(s) = body.structuring_threshold {
        engine.structuring_threshold = s;
    }

    let patterns = engine.analyze(&body.transactions);

    Ok(HttpResponse::Ok().json(ApiResponse::success(
        serde_json::json!({
            "patterns": patterns,
            "transactions_analyzed": body.transactions.len(),
        }),
        trace,
    )))
}
