//! Health sub-module for the Symphony Pulse.
//! Handles Liveness and Readiness probes for infrastructure orchestration.

use axum::{extract::Extension, Json};
use std::sync::Arc;
use std::time::Instant;

use crate::api::{HealthResponse, StandardResponse};
use crate::api::extract_request_id_from_headers;
use crate::engine::coordination::service::BongasEngine;
use axum::http::HeaderMap;

/// Liveness probe - determines if the process is alive.
pub async fn liveness_check(
    Extension(start_time): Extension<Arc<Instant>>,
    headers: HeaderMap,
) -> Json<StandardResponse<HealthResponse>> {
    let request_id = extract_request_id_from_headers(&headers);
    let data = HealthResponse {
        status: "up".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: chrono::Utc::now(),
        uptime_seconds: start_time.elapsed().as_secs(),
    };
    Json(StandardResponse::success(data).with_request_id(request_id))
}

/// Readiness probe - determines if the app is ready for traffic.
pub async fn readiness_check(
    Extension(engine): Extension<Arc<BongasEngine>>,
    Extension(start_time): Extension<Arc<Instant>>,
    headers: HeaderMap,
) -> Json<StandardResponse<HealthResponse>> {
    let request_id = extract_request_id_from_headers(&headers);
    
    // Check if Postgres is reachable via the pool
    let is_db_ready = engine.execution.manager.item_feature_service.pool().check_health().await;
    
    let status = if is_db_ready { "ready" } else { "not_ready" };

    let data = HealthResponse {
        status: status.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: chrono::Utc::now(),
        uptime_seconds: start_time.elapsed().as_secs(),
    };
    Json(StandardResponse::success(data).with_request_id(request_id))
}

/// Generic health check for simple monitoring.
pub async fn health_check(
    Extension(start_time): Extension<Arc<Instant>>,
    headers: HeaderMap,
) -> Json<StandardResponse<HealthResponse>> {
    let request_id = extract_request_id_from_headers(&headers);
    let data = HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: chrono::Utc::now(),
        uptime_seconds: start_time.elapsed().as_secs(),
    };
    Json(StandardResponse::success(data).with_request_id(request_id))
}
