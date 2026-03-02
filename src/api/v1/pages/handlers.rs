//! Page management endpoints and handlers.

use axum::{
    extract::{Path, Extension},
    routing::{get, post},
    Json, Router,
    http::HeaderMap,
};
use std::sync::Arc;
use tracing::info;

use crate::engine::BongasEngine;
use crate::api::models::StandardResponse;
use crate::error::AppError;
use crate::pages::types::{PageLayout, SavePageLayoutRequest};

/// Mount all page management routes.
pub fn routes() -> Router {
    Router::new()
        .route("/", post(save_page_layout))
        .route("/active", get(list_active_pages))
        .route("/{slug}", get(get_page_layout).delete(delete_page_layout))
}

// ─── Helpers ────────────────────────────────────────────────────────────────

fn authorize_admin(headers: &HeaderMap, engine: &BongasEngine) -> Result<(), AppError> {
    let system_key = &engine.config.security.system_api_key;
    
    let provided_key = headers.get("X-Platform-Key")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("Missing X-Platform-Key for admin access".to_string()))?;

    if provided_key != system_key {
        return Err(AppError::Unauthorized("Invalid administrative key".to_string()));
    }

    Ok(())
}

// ─── Handlers ───────────────────────────────────────────────────────────────

/// GET /api/v1/pages/active
async fn list_active_pages(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<Vec<PageLayout>>>, AppError> {
    let request_id = headers.get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();
        
    let layouts = engine.pages.list_active_pages().await?;
    Ok(Json(StandardResponse::success(layouts).with_request_id(request_id)))
}

/// GET /api/v1/pages/:slug
async fn get_page_layout(
    headers: HeaderMap,
    Path(slug): Path<String>,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<PageLayout>>, AppError> {
    let request_id = headers.get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();
        
    authorize_admin(&headers, &engine)?;
    
    let layout = engine.pages.get_layout(&slug).await?
        .ok_or_else(|| AppError::NotFound(format!("Page layout {} not found", slug)))?;
        
    Ok(Json(StandardResponse::success(layout).with_request_id(request_id)))
}

/// POST /api/v1/pages
async fn save_page_layout(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BongasEngine>>,
    Json(payload): Json<SavePageLayoutRequest>,
) -> Result<Json<StandardResponse<PageLayout>>, AppError> {
    let request_id = headers.get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();
        
    authorize_admin(&headers, &engine)?;
    
    let layout = engine.pages.save_layout(payload).await?;
    
    info!(request_id = %request_id, page = %layout.page_slug, "Page layout saved and cache invalidated");
    Ok(Json(StandardResponse::success(layout).with_request_id(request_id)))
}

/// DELETE /api/v1/pages/:slug
async fn delete_page_layout(
    headers: HeaderMap,
    Path(slug): Path<String>,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<bool>>, AppError> {
    let request_id = headers.get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();
        
    authorize_admin(&headers, &engine)?;
    
    let success = engine.pages.delete_layout(&slug).await?;
    
    if !success {
        return Err(AppError::NotFound(format!("Page layout {} not found or already deleted", slug)));
    }
    
    info!(request_id = %request_id, page = %slug, "Page layout soft-deleted");
    Ok(Json(StandardResponse::success(true).with_request_id(request_id)))
}
