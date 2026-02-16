use std::sync::Arc;
use anyhow::Result;
use crate::db::repositories::scenario_repository::ScenarioRepository;
use crate::db::models::ScenarioConfig;

/// Loader for fetching and managing scenario configurations from the database.
pub struct ScenarioLoader {
    repository: Arc<ScenarioRepository>,
}

impl ScenarioLoader {
    /// Create a new ScenarioLoader with the given repository.
    pub fn new(repository: Arc<ScenarioRepository>) -> Self {
        Self { repository }
    }

    /// Load all enabled scenario configurations from the database.
    pub async fn load_all_enabled(&self) -> Result<Vec<ScenarioConfig>> {
        Ok(self.repository.find_all_enabled().await?)
    }

    /// Load a single scenario configuration by its slug.
    pub async fn load_by_slug(&self, slug: &str) -> Result<Option<ScenarioConfig>> {
        Ok(self.repository.find_by_slug(slug).await?)
    }
}
