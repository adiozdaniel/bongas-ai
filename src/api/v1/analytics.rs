use axum::{
    extract::Extension,
    response::Json,
};
use serde_json::json;
use std::sync::Arc;

use crate::api::error::ApiError;
use crate::api::models::ApiResponse;
use crate::analytics::{AnalyticsManager, AnalyticsMetricsSummary};

/// GET /api/v1/analytics/metrics
/// Get all analytics metrics in Prometheus format
pub async fn get_analytics_metrics(
    Extension(analytics): Extension<Arc<AnalyticsManager>>,
) -> Result<Json<ApiResponse<String>>, ApiError> {
    let metrics_data = analytics.get_metrics()
        .map_err(|e| ApiError::Internal(format!("Failed to get metrics: {}", e)))?;
    
    Ok(Json(ApiResponse::success(metrics_data)))
}

/// GET /api/v1/analytics/metrics/summary
/// Get analytics metrics summary
pub async fn get_analytics_metrics_summary(
    Extension(analytics): Extension<Arc<AnalyticsManager>>,
) -> Result<Json<ApiResponse<AnalyticsMetricsSummary>>, ApiError> {
    let summary = analytics.get_summary().await;
    Ok(Json(ApiResponse::success(summary)))
}

/// GET /api/v1/analytics/metrics/health
/// Get analytics health check with metrics
pub async fn get_analytics_health(
    Extension(analytics): Extension<Arc<AnalyticsManager>>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let summary = analytics.get_summary().await;
    
    let health_data = json!({
        "status": "healthy",
        "metrics_available": true,
        "summary": summary,
        "timestamp": chrono::Utc::now().to_rfc3339()
    });
    
    Ok(Json(ApiResponse::success(health_data)))
}