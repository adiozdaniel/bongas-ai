//! Discovery sub-module for the Symphony Backstage.
//! Handles management of device-specific orchestration configurations.

use axum::{
    extract::Extension,
    Json,
    http::HeaderMap,
};
use std::sync::Arc;
use std::collections::HashMap;
use tracing::info;

use crate::engine::coordination::service::BongasEngine;
use crate::api::StandardResponse;
use crate::error::AppError;
use crate::db::DiscoveryConfig;
use crate::api::extract_request_id_from_headers;

/// GET /api/v1/recommendation/admin/discovery/config
/// List all in-memory discovery configurations.
pub async fn list_discovery_configs(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<HashMap<String, DiscoveryConfig>>>, AppError> {
    let request_id = extract_request_id_from_headers(&headers);
    let configs = engine.governance.discovery_configs.read().await.clone();
    Ok(Json(StandardResponse::success(configs).with_request_id(request_id)))
}

/// POST /api/v1/recommendation/admin/discovery/config
/// Update or create a discovery configuration in the database and reload the engine.
pub async fn save_discovery_config(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BongasEngine>>,
    Json(payload): Json<DiscoveryConfig>,
) -> Result<Json<StandardResponse<DiscoveryConfig>>, AppError> {
    let request_id = extract_request_id_from_headers(&headers);
    let config = engine.governance.discovery_repo.upsert(payload).await?;
    
    info!(request_id = %request_id, device = %config.device_type, "Discovery configuration saved/updated");
    
    // Trigger global reload of discovery configurations in memory
    engine.governance.reload_discovery_configs().await?;
    
    Ok(Json(StandardResponse::success(config).with_request_id(request_id)))
}
