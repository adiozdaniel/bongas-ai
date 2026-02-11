use anyhow::Result;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde_json;
use super::bandits::{thompson_sampling::ThompsonSampling, ucb::UCB1, linucb::LinUCB};
use super::ab_testing::ABTestManager;

pub enum BanditAlgorithm {
    ThompsonSampling(ThompsonSampling),
    UCB1(UCB1),
    LinUCB(LinUCB),
}

pub struct ExperimentManager {
    db_pool: Arc<PgPool>,
    bandits: Arc<RwLock<HashMap<String, BanditAlgorithm>>>,
    ab_tests: Arc<RwLock<ABTestManager>>,
}

impl ExperimentManager {
    pub fn new(db_pool: Arc<PgPool>) -> Self {
        Self {
            db_pool,
            bandits: Arc::new(RwLock::new(HashMap::new())),
            ab_tests: Arc::new(RwLock::new(ABTestManager::new())),
        }
    }

    /// Create new Thompson Sampling experiment
    pub async fn create_thompson_sampling(
        &self,
        experiment_id: &str,
        arm_names: Vec<String>,
    ) -> Result<()> {
        let bandit = ThompsonSampling::new(arm_names);
        let mut bandits = self.bandits.write().await;
        bandits.insert(
            experiment_id.to_string(),
            BanditAlgorithm::ThompsonSampling(bandit),
        );

        Ok(())
    }

    /// Select arm from Thompson Sampling
    pub async fn select_arm_thompson(
        &self,
        experiment_id: &str,
    ) -> Result<String> {
        let bandits = self.bandits.read().await;
        let bandit = bandits
            .get(experiment_id)
            .ok_or_else(|| anyhow::anyhow!("Experiment not found"))?;

        match bandit {
            BanditAlgorithm::ThompsonSampling(ts) => ts.select_arm(),
            _ => Err(anyhow::anyhow!("Wrong algorithm type")),
        }
    }

    /// Update Thompson Sampling with reward
    pub async fn update_thompson(
        &self,
        experiment_id: &str,
        arm_name: &str,
        reward: f64,
    ) -> Result<()> {
        let mut bandits = self.bandits.write().await;
        let bandit = bandits
            .get_mut(experiment_id)
            .ok_or_else(|| anyhow::anyhow!("Experiment not found"))?;

        match bandit {
            BanditAlgorithm::ThompsonSampling(ts) => ts.update(arm_name, reward),
            _ => Err(anyhow::anyhow!("Wrong algorithm type")),
        }
    }

    /// Persist experiment results to database
    pub async fn save_results(&self, experiment_id: &str) -> Result<()> {
        let bandits = self.bandits.read().await;
        let bandit = bandits
            .get(experiment_id)
            .ok_or_else(|| anyhow::anyhow!("Experiment not found"))?;

        let stats = match bandit {
            BanditAlgorithm::ThompsonSampling(ts) => {
                serde_json::to_value(ts.get_stats())?
            }
            BanditAlgorithm::UCB1(ucb) => {
                serde_json::to_value(ucb.get_stats())?
            }
            _ => serde_json::Value::Null,
        };

        sqlx::query(
            "INSERT INTO experiments (experiment_id, results, updated_at)
            VALUES ($1, $2, NOW())
            ON CONFLICT (experiment_id)
            DO UPDATE SET results = EXCLUDED.results, updated_at = NOW()"
        )
        .bind(experiment_id)
        .bind(stats)
        .execute(self.db_pool.as_ref())
        .await?;

        Ok(())
    }
}