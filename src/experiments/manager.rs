use anyhow::{Result, anyhow};
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use super::bandits::{thompson_sampling::ThompsonSampling, ucb::UCB1, linucb::LinUCB};
use super::ab_testing::ABTestManager;

pub enum BanditAlgorithm {
    ThompsonSampling(ThompsonSampling),
    UCB1(UCB1),
    LinUCB(LinUCB),
}

pub struct ExperimentManager {
    _db_pool: Arc<PgPool>,
    bandits: Arc<RwLock<HashMap<String, BanditAlgorithm>>>,
    _ab_tests: Arc<RwLock<ABTestManager>>,
}

impl ExperimentManager {
    pub fn new(db_pool: Arc<PgPool>) -> Self {
        Self {
            _db_pool: db_pool,
            bandits: Arc::new(RwLock::new(HashMap::new())),
            _ab_tests: Arc::new(RwLock::new(ABTestManager::new())),
        }
    }

    /// Create new experiment with specified algorithm (supports all 3)
    pub async fn create_experiment(
        &self,
        experiment_id: &str,
        algorithm: &str,
        arm_names: Vec<String>,
    ) -> Result<()> {
        
        
        let mut bandits = self.bandits.write().await;
        
        match algorithm.to_lowercase().as_str() {
            "thompson_sampling" | "thompson" => {
                let bandit = ThompsonSampling::new(arm_names);
                bandits.insert(experiment_id.to_string(), BanditAlgorithm::ThompsonSampling(bandit));
                info!(experiment_id = experiment_id, algorithm = algorithm, "Thompson Sampling experiment created");
            }
            "ucb1" | "ucb" => {
                // UCB1 uses default exploration constant of 2.0
                let bandit = UCB1::new(arm_names.clone(), 2.0);
                bandits.insert(experiment_id.to_string(), BanditAlgorithm::UCB1(bandit));
                info!(experiment_id = experiment_id, algorithm = algorithm, "UCB1 experiment created");
            }
            "linucb" => {
                // LinUCB uses default feature_dim=10, alpha=1.0
                // Note: LinUCB requires ContextFeatures for select/update, using simplified version
                let feature_dim = 10;
                let alpha = 1.0;
                let mut arms = std::collections::HashMap::new();
                for name in &arm_names {
                    arms.insert(name.clone(), 
                        crate::experiments::bandits::linucb::LinUCBArm::new(feature_dim));
                }
                let bandit = crate::experiments::bandits::linucb::LinUCB {
                    arms,
                    alpha,
                    feature_dim,
                };
                bandits.insert(experiment_id.to_string(), BanditAlgorithm::LinUCB(bandit));
                info!(experiment_id = experiment_id, algorithm = algorithm, "LinUCB experiment created");
            }
            _ => return Err(anyhow!("Unknown algorithm: {}", algorithm)),
        }

        Ok(())
    }

    /// Select arm using the experiment's algorithm
    pub async fn select_arm(&self, experiment_id: &str) -> Result<String> {
        let bandits = self.bandits.read().await;
        let bandit = bandits
            .get(experiment_id)
            .ok_or_else(|| anyhow!("Experiment not found: {}", experiment_id))?;

        match bandit {
            BanditAlgorithm::ThompsonSampling(ts) => ts.select_arm(),
            BanditAlgorithm::UCB1(ucb) => ucb.select_arm(),
            BanditAlgorithm::LinUCB(linucb) => linucb.select_arm(),
        }
    }

    /// Update arm with reward
    pub async fn update_arm(
        &self,
        experiment_id: &str,
        arm_name: &str,
        reward: f64,
    ) -> Result<()> {
        let mut bandits = self.bandits.write().await;
        let bandit = bandits
            .get_mut(experiment_id)
            .ok_or_else(|| anyhow!("Experiment not found: {}", experiment_id))?;

        match bandit {
            BanditAlgorithm::ThompsonSampling(ts) => ts.update(arm_name, reward),
            BanditAlgorithm::UCB1(ucb) => ucb.update(arm_name, reward),
            BanditAlgorithm::LinUCB(linucb) => linucb.update(arm_name, reward),
        }
    }

    /// Get experiment status
    pub async fn get_status(&self, experiment_id: &str) -> Result<serde_json::Value> {
        use serde_json::json;

        let bandits = self.bandits.read().await;
        let bandit = bandits
            .get(experiment_id)
            .ok_or_else(|| anyhow!("Experiment not found: {}", experiment_id))?;

        let stats = match bandit {
            BanditAlgorithm::ThompsonSampling(ts) => json!({
                "algorithm": "thompson_sampling",
                "stats": ts.get_stats()
            }),
            BanditAlgorithm::UCB1(ucb) => json!({
                "algorithm": "ucb1",
                "stats": ucb.get_stats()
            }),
            BanditAlgorithm::LinUCB(linucb) => {
                // LinUCB doesn't have get_stats, create manual stats
                let arm_stats: serde_json::Map<String, serde_json::Value> = linucb.arms
                    .iter()
                    .map(|(name, arm)| {
                        (name.clone(), json!({
                            "A_determinant": arm.a.determinant(),
                            "d": arm.dim
                        }))
                    })
                    .collect();
                json!({
                    "algorithm": "linucb",
                    "arms": arm_stats,
                    "alpha": linucb.alpha,
                    "feature_dim": linucb.feature_dim
                })
            }
        };

        Ok(stats)
    }

    /// List all running experiments
    pub async fn list_experiments(&self) -> Vec<String> {
        let bandits = self.bandits.read().await;
        bandits.keys().cloned().collect()
    }

    /// Load experiments from database configuration
    pub async fn load_from_database(&self) -> Result<usize> {
        info!("Loading experiments from database...");

        // Query experiments table using runtime query (no DATABASE_URL needed at compile time)
        let rows = sqlx::query(
            r#"
            SELECT experiment_id, algorithm, arm_names, enabled
            FROM experiments_config
            WHERE enabled = true
            "#
        )
        .fetch_all(self._db_pool.as_ref())
        .await?;

        let mut loaded = 0;
        for row in rows {
            let experiment_id: String = row.get("experiment_id");
            let arm_names_json: Option<String> = row.get("arm_names");
            let algorithm: Option<String> = row.get("algorithm");

            if let Some(arms_json) = arm_names_json {
                match serde_json::from_str::<Vec<String>>(&arms_json) {
                    Ok(arm_names) => {
                        if !arm_names.is_empty() {
                            let alg = algorithm.unwrap_or_else(|| "thompson_sampling".to_string());

                            self.create_experiment(
                                &experiment_id,
                                &alg,
                                arm_names,
                            ).await?;
                            loaded += 1;
                        }
                    }
                    Err(e) => {
                        warn!(error = %e, "Failed to parse arm names for experiment {}", experiment_id);
                    }
                }
            }
        }

        if loaded == 0 {
            warn!("No experiments found in database, using defaults");
            // Create default experiment if none exist
            self.create_experiment(
                "default_experiment",
                "thompson_sampling",
                vec!["personalized_home".to_string(), "trending_home".to_string()],
            ).await?;
            loaded = 1;
        }

        Ok(loaded)
    }

    /// Persist experiment results to database
    pub async fn save_results(&self, experiment_id: &str) -> Result<()> {
        let status = self.get_status(experiment_id).await?;

        sqlx::query(
            "INSERT INTO experiments (experiment_id, results, updated_at)
            VALUES ($1, $2, NOW())
            ON CONFLICT (experiment_id)
            DO UPDATE SET results = EXCLUDED.results, updated_at = NOW()"
        )
        .bind(experiment_id)
        .bind(status)
        .execute(self._db_pool.as_ref())
        .await?;

        Ok(())
    }
}
