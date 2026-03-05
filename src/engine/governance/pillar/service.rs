use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
pub use crate::engine::governance::orchestration::manager::service::PagesManager;
pub use crate::engine::governance::factory::scenarios_manager::service::ScenariosManager;
pub use crate::engine::governance::factory::scenario_factory::service::ScenarioFactory;
pub use crate::db::repositories::discovery_repository::service::DiscoveryConfigRepository;
pub use crate::db::DiscoveryConfig;

/// 🔐 THE BACKSTAGE: Administrative governance.
pub struct GovernancePillar {
    pub orchestration: Arc<PagesManager>,
    pub scenarios: Arc<ScenariosManager>,
    pub scenario_factory: Arc<ScenarioFactory>,
    pub discovery_repo: Arc<DiscoveryConfigRepository>,
    pub discovery_configs: Arc<RwLock<HashMap<String, DiscoveryConfig>>>,
}

impl GovernancePillar {
    pub fn new(
        orchestration: Arc<PagesManager>,
        scenarios: Arc<ScenariosManager>,
        scenario_factory: Arc<ScenarioFactory>,
        discovery_repo: Arc<DiscoveryConfigRepository>,
    ) -> Self {
        Self { 
            orchestration, 
            scenarios, 
            scenario_factory,
            discovery_repo,
            discovery_configs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Hydrate or refresh the in-memory discovery configurations.
    pub async fn reload_discovery_configs(&self) -> crate::error::AppResult<usize> {
        let configs = self.discovery_repo.find_all().await?;
        let count = configs.len();
        
        let mut lock = self.discovery_configs.write().await;
        lock.clear();
        for config in configs {
            lock.insert(config.device_type.clone(), config);
        }
        
        tracing::info!(count, "Discovery configurations reloaded into memory");
        Ok(count)
    }

    /// Get a discovery configuration for a specific device, with fallback to 'default'.
    pub async fn get_discovery_config(&self, device_type: Option<&str>) -> DiscoveryConfig {
        let lock = self.discovery_configs.read().await;
        
        let device = device_type.unwrap_or("default");
        
        lock.get(device)
            .or_else(|| lock.get("default"))
            .cloned()
            .unwrap_or_else(|| DiscoveryConfig {
                device_type: "fallback".to_string(),
                initial_batch_size: 5,
                continuation_batch_size: 5,
                prewarm_lookahead: 5,
                ghost_ttl_seconds: 300,
                cache_ttl_seconds: 300,
                updated_at: chrono::Utc::now(),
            })
    }
}
