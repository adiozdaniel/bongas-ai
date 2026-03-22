use anyhow::Result;
use std::collections::HashMap;
use tracing::{info, warn};

use crate::db::ScenarioRepository;
use crate::db::{PipelineDefinition, ScenarioWithStrategy};
use crate::engine::governance::strategy::resolver::service::ActiveRule;
use crate::engine::coordination::service::ScenarioDefinition;

pub struct ScenarioFactory {
    pub repo: ScenarioRepository
}

type RawScenarioRow = (i32, String, i32, Option<String>, Option<String>, serde_json::Value, String, serde_json::Value);

impl ScenarioFactory {
    pub fn new(repo: ScenarioRepository) -> Self {
        Self { repo }
    }

    pub fn repo(&self) -> &ScenarioRepository {
        &self.repo
    }

    /// Load all active rules for the Strategy Resolver.
    pub async fn load_all_rules(&self) -> Result<HashMap<String, Vec<ActiveRule>>> {
        info!("Loading strategic rules from database...");

        // Query joining rules, pipelines, and scenarios
        let rows: Vec<RawScenarioRow> = self.repo.pool().execute(|pool| async move {
            sqlx::query_as::<_, RawScenarioRow>(
                r#"
                SELECT 
                    r.id, 
                    s.slug as scenario_slug, 
                    r.priority, 
                    r.device_type,
                    r.maturity_rating,
                    r.condition, 
                    p.slug as pipeline_slug, 
                    p.definition as pipeline_definition
                FROM bongas.scenario_rules r
                JOIN bongas.scenarios s ON r.scenario_id = s.id
                JOIN bongas.pipelines p ON r.pipeline_id = p.id
                WHERE r.is_active = true
                ORDER BY s.slug, r.priority DESC
                "#
            )
            .fetch_all(&pool)
            .await
        })
        .await?;

        let mut rule_map: HashMap<String, Vec<ActiveRule>> = HashMap::new();

        for (id, s_slug, priority, device_type, maturity_rating, condition, p_slug, p_def) in rows {
            let pipeline_definition: PipelineDefinition = serde_json::from_value(p_def)?;
            
            let rule = ActiveRule {
                id,
                priority,
                device_type,
                maturity_rating,
                condition,
                pipeline_slug: p_slug,
                pipeline_definition,
                pipeline: None, // Will be linked by BongasEngine
            };

            rule_map.entry(s_slug).or_default().push(rule);
        }

        info!(count = rule_map.values().flatten().count(), "Strategic rules loaded from database");
        Ok(rule_map)
    }

    /// Load all enabled scenarios from database
    pub async fn load_all_from_db(&self) -> Result<HashMap<String, ScenarioDefinition>> {
        info!("Loading scenarios from database...");

        let configs = self.repo.find_all_active().await?;

        let mut scenarios = HashMap::new();
        let mut candle_count = 0;

        for config in configs {
            match self.parse_scenario(&config) {
                Ok(scenario) => {
                    let uses_candle = scenario.pipeline.stages.iter()
                        .any(|stage| stage.r#type.starts_with("candle_") || stage.r#type.starts_with("onnx_"));

                    if uses_candle {
                        candle_count += 1;
                    }

                    info!(slug = %scenario.slug, uses_candle = uses_candle, "Loaded scenario");
                    scenarios.insert(scenario.slug.clone(), scenario);
                }
                Err(e) => {
                    warn!(slug = %config.scenario.slug, error = %e, "Failed to parse scenario");
                }
            }
        }

        info!(
            total = scenarios.len(),
            candle_enabled = candle_count,
            "Scenarios loaded"
        );

        Ok(scenarios)
    }

    /// Load a single scenario from database by slug
    pub async fn load_one_from_db(&self, slug: &str) -> Result<Option<ScenarioDefinition>> {
        if let Some(config) = self.repo.find_by_slug(slug).await? {
            let definition = self.parse_scenario(&config)?;
            Ok(Some(definition))
        } else {
            Ok(None)
        }
    }

    /// Parse scenario config into scenario definition
    fn parse_scenario(&self, config: &ScenarioWithStrategy) -> Result<ScenarioDefinition> {
        Ok(ScenarioDefinition {
            slug: config.scenario.slug.clone(),
            name: config.scenario.name.clone(),
            pipeline: config.pipeline.clone(),
            maturity_rating: config.scenario.maturity_rating.clone(),
            cache_ttl_seconds: config.scenario.cache_ttl_seconds,
            use_l2_cache: config.scenario.use_l2_cache,
            initial_display_limit: config.scenario.initial_display_limit,
            scope: config.scenario.scope.clone(),
            linked_pipeline: None,
        })
    }
}
