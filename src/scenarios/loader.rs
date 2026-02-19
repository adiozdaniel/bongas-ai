use std::sync::Arc;
use anyhow::Result;
use crate::db::repositories::scenario_repository::ScenarioRepository;
use crate::db::models::ScenarioWithStrategy;

/// Loader for fetching and managing scenario configurations from the database.
pub struct ScenarioLoader {
    repository: Arc<ScenarioRepository>,
}

impl ScenarioLoader {
    /// Create a new ScenarioLoader with the given repository.
    pub fn new(repository: Arc<ScenarioRepository>) -> Self {
        Self { repository }
    }

    /// Load all active scenario configurations from the database.
    pub async fn load_all_active(&self) -> Result<Vec<ScenarioWithStrategy>> {
        Ok(self.repository.find_all_active().await?)
    }

    /// Load a single scenario configuration and strategy by its slug.
    pub async fn load_by_slug(&self, slug: &str) -> Result<Option<ScenarioWithStrategy>> {
        Ok(self.repository.find_by_slug(slug).await?)
    }
}
