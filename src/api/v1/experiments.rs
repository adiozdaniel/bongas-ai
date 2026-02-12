use axum::extract::{Extension, Json, Path};
use std::sync::Arc;
use tracing::{info, error};
use serde::{Deserialize, Serialize};
use anyhow::Context;
use crate::engine::{BongasEngine, RecommendationItem};
use crate::api::models::ApiResponse;

/// Request/Response models
#[derive(Debug, Deserialize)]
pub struct CreateExperimentRequest {
    pub experiment_id: String,
    pub algorithm: String,
    pub arms: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ExperimentResponse {
    pub experiment_id: String,
    pub algorithm: String,
    pub arms: Vec<String>,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct RecordRewardRequest {
    pub scenario_slug: String,
    pub reward_type: String,
    pub reward_value: f64,
}

/// POST /api/v1/experiments
/// Create a new experiment with arms (scenario variants)
pub async fn create_experiment(
    Extension(engine): Extension<Arc<BongasEngine>>,
    Json(req): Json<CreateExperimentRequest>,
) -> Result<Json<ApiResponse<ExperimentResponse>>, String> {
    info!(
        experiment_id = %req.experiment_id,
        algorithm = %req.algorithm,
        arms = ?req.arms,
        "Creating experiment"
    );

    if req.arms.is_empty() {
        return Err("At least one arm is required".to_string());
    }

    match engine.create_experiment(
        &req.experiment_id,
        &req.algorithm,
        req.arms.clone(),
    ).await {
        Ok(_) => Ok(Json(ApiResponse::success(ExperimentResponse {
            experiment_id: req.experiment_id,
            algorithm: req.algorithm,
            arms: req.arms,
            status: "created".to_string(),
        }))),
        Err(e) => {
            error!(error = %e, "Failed to create experiment");
            Err(format!("Failed to create experiment: {}", e))
        }
    }
}

/// GET /api/v1/experiments
/// List all running experiments
pub async fn list_experiments(
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<ApiResponse<Vec<String>>>, String> {
    let experiments = engine.list_experiments().await;
    info!(count = experiments.len(), "Listing experiments");
    Ok(Json(ApiResponse::success(experiments)))
}

/// GET /api/v1/experiments/:id
/// Get experiment details
pub async fn get_experiment(
    Extension(engine): Extension<Arc<BongasEngine>>,
    Path(experiment_id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, String> {
    match engine.get_experiment_status(&experiment_id).await {
        Ok(status) => {
            let response = serde_json::json!({
                "experiment_id": experiment_id,
                "status": status
            });
            Ok(Json(ApiResponse::success(response)))
        }
        Err(e) => Err(format!("Experiment not found: {}", e)),
    }
}

/// GET /api/v1/experiments/:id/status
/// Get experiment statistics
pub async fn get_experiment_status(
    Extension(engine): Extension<Arc<BongasEngine>>,
    Path(experiment_id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, String> {
    match engine.get_experiment_status(&experiment_id).await {
        Ok(status) => Ok(Json(ApiResponse::success(status))),
        Err(e) => Err(format!("Experiment not found: {}", e)),
    }
}

/// POST /api/v1/experiments/:id/execute/:user_id
/// Execute scenario selected by bandit
pub async fn execute_experiment(
    Extension(engine): Extension<Arc<BongasEngine>>,
    Path((experiment_id, user_id)): Path<(String, i32)>,
) -> Result<Json<ApiResponse<Vec<RecommendationItem>>>, String> {
    info!(
        experiment_id = %experiment_id,
        user_id = user_id,
        "Executing experiment"
    );

    let items = engine.execute_scenario_with_experiment(
        &experiment_id,
        Some(user_id),
        serde_json::json!({}),
    )
    .await
    .context("Failed to execute experiment")
    .map_err(|e| format!("Failed to execute experiment: {}", e))?;
    
    Ok(Json(ApiResponse::success(items)))
}

/// POST /api/v1/experiments/:id/reward
/// Record reward for bandit learning (supports completion, click, like, dislike)
pub async fn record_reward(
    Extension(engine): Extension<Arc<BongasEngine>>,
    Path(experiment_id): Path<String>,
    Json(req): Json<RecordRewardRequest>,
) -> Result<Json<ApiResponse<()>>, String> {
    info!(
        experiment_id = %experiment_id,
        scenario = %req.scenario_slug,
        reward_type = %req.reward_type,
        reward_value = req.reward_value,
        "Recording experiment reward"
    );

    engine.record_experiment_reward(
        &experiment_id,
        &req.scenario_slug,
        &req.reward_type,
        req.reward_value,
    )
    .await
    .context("Failed to record reward")
    .map_err(|e| format!("Failed to record reward: {}", e))?;
    
    Ok(Json(ApiResponse::success(())))
}

/// POST /api/v1/experiments/load
/// Load experiments from database configuration
pub async fn load_experiments(
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<ApiResponse<usize>>, String> {
    info!("Loading experiments from database");
    
    let count = engine.load_experiments_from_db().await
        .context("Failed to load experiments")
        .map_err(|e| format!("Failed to load experiments: {}", e))?;
    
    info!(count = count, "Experiments loaded from database");
    Ok(Json(ApiResponse::success(count)))
}
