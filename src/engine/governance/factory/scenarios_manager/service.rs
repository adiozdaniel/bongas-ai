//! Scenario lifecycle management and governance.

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use tokio::sync::RwLock;
use arc_swap::ArcSwap;
use tracing::{info, warn};

use crate::pipeline::{ExecutablePipeline, executor::PipelineExecutor};
use crate::engine::engine::types::ScenarioDefinition;
use crate::engine::scenario_factory::ScenarioFactory;
use crate::engine::staging_manager::StagingManager;
use crate::engine::strategy_resolver::StrategyResolver;

pub struct ScenariosManager {
    pub(crate) scenarios: Arc<RwLock<HashMap<String, ScenarioDefinition>>>,
    pub(crate) linked_scenarios: Arc<ArcSwap<HashMap<String, Arc<ExecutablePipeline>>>>,
    pub(crate) scenario_factory: Arc<ScenarioFactory>,
    pub(crate) pipeline_executor: Arc<PipelineExecutor>,
    pub(crate) strategy_resolver: Arc<StrategyResolver>,
    pub(crate) staging_manager: Arc<StagingManager>,
    pub(crate) max_active_scenarios: Arc<AtomicUsize>,
}

impl ScenariosManager {
    pub fn new(
        scenario_factory: Arc<ScenarioFactory>,
        pipeline_executor: Arc<PipelineExecutor>,
        strategy_resolver: Arc<StrategyResolver>,
        staging_manager: Arc<StagingManager>,
        max_scenarios: usize,
    ) -> Self {
        Self {
            scenarios: Arc::new(RwLock::new(HashMap::new())),
            linked_scenarios: Arc::new(ArcSwap::new(Arc::new(HashMap::new()))),
            scenario_factory,
            pipeline_executor,
            strategy_resolver,
            staging_manager,
            max_active_scenarios: Arc::new(AtomicUsize::new(max_scenarios)),
        }
    }

    /// HOT-RELOAD: Reload all scenarios and strategic rules from database.
    pub async fn reload_scenarios(&self) -> Result<usize> {
        info!("Reloading scenarios and strategic rules from database...");

        self.staging_manager.cleanup_locks();

        let mut new_scenarios: HashMap<String, ScenarioDefinition> = self.scenario_factory.load_all_from_db().await?;

        // ENFORCE GOVERNANCE
        let max_scenarios = self.max_active_scenarios.load(std::sync::atomic::Ordering::Relaxed);
        if new_scenarios.len() > max_scenarios {
            warn!(total = new_scenarios.len(), limit = max_scenarios, "Scenario count exceeds governance limit. Truncating.");
            let mut keys: Vec<String> = new_scenarios.keys().cloned().collect();
            keys.sort(); 
            keys.truncate(max_scenarios);
            new_scenarios.retain(|k, _| keys.contains(k));
        }
        
        let mut rule_map = self.scenario_factory.load_all_rules().await?;
        let mut linked_map = HashMap::new();

        // Link pipelines
        for scenario in new_scenarios.values_mut() {
            if let Ok(executable) = self.pipeline_executor.link(&scenario.pipeline) {
                let arc_executable = Arc::new(executable);
                scenario.linked_pipeline = Some(arc_executable.clone());
                linked_map.insert(scenario.slug.clone(), arc_executable);
            }
        }

        for rules in rule_map.values_mut() {
            for rule in rules {
                if let Ok(executable) = self.pipeline_executor.link(&rule.pipeline_definition) {
                    rule.pipeline = Some(Arc::new(executable));
                }
            }
        }

        // Atomic Updates
        let mut scenarios = self.scenarios.write().await;
        scenarios.clear();
        scenarios.extend(new_scenarios);

        self.linked_scenarios.store(Arc::new(linked_map));
        self.strategy_resolver.update_rules(rule_map);

        Ok(scenarios.len())
    }

    pub async fn reload_scenario(&self, slug: &str) -> Result<bool> {
        if let Some(mut new_scenario) = self.scenario_factory.load_one_from_db(slug).await? {
            let mut scenarios = self.scenarios.write().await;

            if let Ok(executable) = self.pipeline_executor.link(&new_scenario.pipeline) {
                let arc_executable = Arc::new(executable);
                new_scenario.linked_pipeline = Some(arc_executable.clone());
                
                let mut linked_map = (**self.linked_scenarios.load()).clone();
                linked_map.insert(slug.to_string(), arc_executable);
                self.linked_scenarios.store(Arc::new(linked_map));
            }

            scenarios.insert(slug.to_string(), new_scenario);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn remove_scenario(&self, slug: &str) {
        let mut scenarios = self.scenarios.write().await;
        if scenarios.remove(slug).is_some() {
            let mut linked_map = (**self.linked_scenarios.load()).clone();
            linked_map.remove(slug);
            self.linked_scenarios.store(Arc::new(linked_map));
        }
    }

    pub fn scenario_count(&self) -> usize {
        // Simple synchronous approximation if needed, or use a task
        0 // Placeholder for actual implementation
    }
}
