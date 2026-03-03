//! Orchestration sub-module for the Symphony Backstage.
//! Handles SDUI Layout management and Navigation Mesh configuration.

use axum::{
    extract::{Path, Extension},
    Json,
    http::HeaderMap,
};
use std::sync::Arc;
use tracing::info;

use crate::engine::BongasEngine;
use crate::api::models::StandardResponse;
use crate::error::AppError;
use crate::engine::governance::orchestration::types::{PageLayout, SavePageLayoutRequest};
use crate::api::middleware::service::extract_request_id_from_headers;

/// GET /api/v1/recommendation/admin/pages/active
pub async fn list_active_pages(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<Vec<PageLayout>>>, AppError> {
    let request_id = extract_request_id_from_headers(&headers);
    let layouts = engine.governance.orchestration.list_active_pages().await?;
    Ok(Json(StandardResponse::success(layouts).with_request_id(request_id)))
}

/// GET /api/v1/recommendation/admin/pages/:slug
pub async fn get_page_layout(
    headers: HeaderMap,
    Path(slug): Path<String>,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<PageLayout>>, AppError> {
    let request_id = extract_request_id_from_headers(&headers);
    
    let layout: PageLayout = engine.governance.orchestration.get_layout_contextual(&slug, None, None, None).await?
        .ok_or_else(|| AppError::NotFound(format!("Page layout {} not found", slug)))?;
        
    Ok(Json(StandardResponse::success(layout).with_request_id(request_id)))
}

/// POST /api/v1/recommendation/admin/pages
pub async fn save_page_layout(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BongasEngine>>,
    Json(payload): Json<SavePageLayoutRequest>,
) -> Result<Json<StandardResponse<PageLayout>>, AppError> {
    let request_id = extract_request_id_from_headers(&headers);
    let layout = engine.governance.orchestration.save_layout(payload).await?;
    
    info!(request_id = %request_id, page = %layout.page_slug, "Page layout saved and cache invalidated");
    Ok(Json(StandardResponse::success(layout).with_request_id(request_id)))
}

/// DELETE /api/v1/recommendation/admin/pages/:slug
pub async fn delete_page_layout(
    headers: HeaderMap,
    Path(slug): Path<String>,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<bool>>, AppError> {
    let request_id = extract_request_id_from_headers(&headers);
    let success = engine.governance.orchestration.delete_layout(&slug).await?;
    
    if !success {
        return Err(AppError::NotFound(format!("Page layout {} not found or already deleted", slug)));
    }
    
    info!(request_id = %request_id, page = %slug, "Page layout soft-deleted");
    Ok(Json(StandardResponse::success(true).with_request_id(request_id)))
}
