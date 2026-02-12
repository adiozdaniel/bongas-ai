use anyhow::Result;
use std::sync::Arc;
use std::collections::HashMap;
use sqlx::PgPool;
use tracing::{info, warn};

use crate::db::repositories::scenario_repository::ScenarioRepository;
use crate::db::models::PipelineDefinition;
use super::ScenarioDefinition;

pub struct ScenarioFactory {
    repo: ScenarioRepository,
}

impl ScenarioFactory {
    pub fn new(db_pool: Arc<PgPool>) -> Self {
        Self {
            repo: ScenarioRepository::new(db_pool.as_ref().clone()),
        }
    }

    /// Load all enabled scenarios from database
    pub async fn load_all_from_db(&self) -> Result<HashMap<String, ScenarioDefinition>> {
        info!("Loading scenarios from database...");

        let configs = self.repo.find_all_enabled().await?;

        let mut scenarios = HashMap::new();
        let mut onnx_count = 0;

        for config in configs {
            match self.parse_scenario(&config) {
                Ok(scenario) => {
                    let uses_onnx = scenario.pipeline.stages.iter()
                        .any(|stage| stage.r#type.starts_with("onnx_"));

                    if uses_onnx {
                        onnx_count += 1;
                    }

                    info!(slug = %scenario.slug, uses_onnx = uses_onnx, "Loaded scenario");
                    scenarios.insert(scenario.slug.clone(), scenario);
                }
                Err(e) => {
                    warn!(slug = %config.slug, error = %e, "Failed to parse scenario");
                }
            }
        }

        info!(
            total = scenarios.len(),
            onnx_enabled = onnx_count,
            "Scenarios loaded"
        );

        Ok(scenarios)
    }

    /// Parse scenario config into scenario definition
    fn parse_scenario(&self, config: &crate::db::models::ScenarioConfig) -> Result<ScenarioDefinition> {
        let pipeline: PipelineDefinition = serde_json::from_value(config.pipeline.clone())?;

        Ok(ScenarioDefinition {
            slug: config.slug.clone(),
            pipeline,
            cache_ttl_seconds: config.cache_ttl_seconds.unwrap_or(300),
            use_l2_cache: config.use_l2_cache,
        })
    }
}
