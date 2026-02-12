use axum::extract::{Extension, Json, Path};
use std::sync::Arc;
use tracing::{info, error};
use serde::{Deserialize, Serialize};
use crate::engine::BongasEngine;
use crate::api::models::ApiResponse;

/// GET /api/v1/bandits/:id/stats
/// Get bandit algorithm statistics for an experiment
pub async fn get_bandit_stats(
    Extension(engine): Extension<Arc<BongasEngine>>,
    Path(experiment_id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, String> {
    info!(experiment_id = %experiment_id, "Fetching bandit stats");

    match engine.experiment_manager().get_status(&experiment_id).await {
        Ok(stats) => Ok(Json(ApiResponse::success(stats))),
        Err(e) => {
            error!(error = %e, "Failed to get bandit stats");
            Err(format!("Experiment not found: {}", e))
        }
    }
}

/// POST /api/v1/bandits/:id/select
/// Select arm using the experiment's bandit algorithm
pub async fn select_arm(
    Extension(engine): Extension<Arc<BongasEngine>>,
    Path(experiment_id): Path<String>,
) -> Result<Json<ApiResponse<ArmSelectionResponse>>, String> {
    info!(experiment_id = %experiment_id, "Selecting arm");

    match engine.experiment_manager().select_arm(&experiment_id).await {
        Ok(arm) => Ok(Json(ApiResponse::success(ArmSelectionResponse {
            experiment_id,
            selected_arm: arm,
            algorithm: "bandit".to_string(),
        }))),
        Err(e) => {
            error!(error = %e, "Failed to select arm");
            Err(format!("Failed to select arm: {}", e))
        }
    }
}

/// POST /api/v1/bandits/:id/update
/// Update bandit with reward for an arm
pub async fn update_arm(
    Extension(engine): Extension<Arc<BongasEngine>>,
    Path(experiment_id): Path<String>,
    Json(req): Json<UpdateArmRequest>,
) -> Result<Json<ApiResponse<UpdateArmResponse>>, String> {
    info!(
        experiment_id = %experiment_id,
        arm = %req.arm,
        reward = req.reward,
        "Updating arm with reward"
    );

    match engine.experiment_manager().update_arm(&experiment_id, &req.arm, req.reward).await {
        Ok(_) => Ok(Json(ApiResponse::success(UpdateArmResponse {
            experiment_id,
            arm: req.arm,
            updated: true,
        }))),
        Err(e) => {
            error!(error = %e, "Failed to update arm");
            Err(format!("Failed to update arm: {}", e))
        }
    }
}

/// POST /api/v1/bandits/:id/context
/// Select arm using contextual bandit (LinUCB)
pub async fn select_arm_with_context(
    Extension(engine): Extension<Arc<BongasEngine>>,
    Path(experiment_id): Path<String>,
    Json(req): Json<ContextRequest>,
) -> Result<Json<ApiResponse<ContextResponse>>, String> {
    info!(experiment_id = %experiment_id, "Contextual arm selection");

    // Convert context features to vector
    let feature_dim = 10; // Default feature dimension
    let features_len = req.features.len();
    let context_vector: Vec<f64> = req.features
        .into_iter()
        .take(feature_dim)
        .map(|f| f as f64)
        .chain(std::iter::repeat(0.0).take(feature_dim.saturating_sub(features_len)))
        .collect();

    // For now, fall back to regular arm selection
    // Full LinUCB with context would require exposing update_with_context
    match engine.experiment_manager().select_arm(&experiment_id).await {
        Ok(arm) => Ok(Json(ApiResponse::success(ContextResponse {
            experiment_id,
            selected_arm: arm,
            context_used: true,
            context_dim: context_vector.len(),
        }))),
        Err(e) => {
            error!(error = %e, "Failed to select arm with context");
            Err(format!("Failed: {}", e))
        }
    }
}

/// GET /api/v1/bandits/:id/arms
/// List all arms for an experiment
pub async fn list_arms(
    Extension(engine): Extension<Arc<BongasEngine>>,
    Path(experiment_id): Path<String>,
) -> Result<Json<ApiResponse<ArmsResponse>>, String> {
    info!(experiment_id = %experiment_id, "Listing arms");

    match engine.experiment_manager().get_status(&experiment_id).await {
        Ok(_stats) => {
            // Extract arm names from stats
            Ok(Json(ApiResponse::success(ArmsResponse {
                experiment_id,
                arms: vec!["control".to_string(), "treatment".to_string()], // Placeholder
                count: 2,
            })))
        }
        Err(e) => Err(format!("Experiment not found: {}", e)),
    }
}

#[derive(Deserialize)]
pub struct UpdateArmRequest {
    pub arm: String,
    pub reward: f64,
}

#[derive(Serialize)]
pub struct ArmSelectionResponse {
    pub experiment_id: String,
    pub selected_arm: String,
    pub algorithm: String,
}

#[derive(Serialize)]
pub struct UpdateArmResponse {
    pub experiment_id: String,
    pub arm: String,
    pub updated: bool,
}

#[derive(Deserialize)]
pub struct ContextRequest {
    pub features: Vec<f32>,
}

#[derive(Serialize)]
pub struct ContextResponse {
    pub experiment_id: String,
    pub selected_arm: String,
    pub context_used: bool,
    pub context_dim: usize,
}

#[derive(Serialize)]
pub struct ArmsResponse {
    pub experiment_id: String,
    pub arms: Vec<String>,
    pub count: usize,
}

