//! High-performance activity processing orchestration.

use std::sync::Arc;
use tokio::sync::{mpsc, Semaphore};
use tracing::{info, error};

use crate::db::ResilientPool;
use crate::db::repositories::interaction_repository::InteractionRepository;
use crate::engine::intelligence::monitoring::staleness_engine::service::{StalenessEngine, UserEvent};
use crate::resilience::ResilienceMetricsCollector;

use crate::ingestion::UserActivity;
use clickhouse::Row;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Row)]
pub struct ClickHouseInteraction {
    pub user_id: i32,
    pub item_id: i32,
    pub interaction_type: String,
    pub scenario_slug: String,
    pub weight: f32,
    pub watch_duration_seconds: i32,
    pub created_at: u64, 
}

use tokio::task::JoinSet;
use crate::engine::governance::orchestration::manager::service::PagesManager;

/// Processes activities from any source and routes them to DB + staleness engine + PagesManager.
pub struct ActivityProcessor {
    interaction_repo: Arc<InteractionRepository>,
    _pool: Arc<ResilientPool>,
    clickhouse: Option<Arc<clickhouse::Client>>,
    staleness_engine: Arc<StalenessEngine>,
    pages_manager: Arc<PagesManager>,
    metrics: Arc<ResilienceMetricsCollector>,
    concurrency_limit: Arc<Semaphore>,
}

impl ActivityProcessor {
    pub fn new(
        interaction_repo: Arc<InteractionRepository>,
        pool: Arc<ResilientPool>,
        clickhouse: Option<Arc<clickhouse::Client>>,
        staleness_engine: Arc<StalenessEngine>,
        pages_manager: Arc<PagesManager>,
        metrics: Arc<ResilienceMetricsCollector>,
        max_concurrency: usize,
    ) -> Self {
        Self {
            interaction_repo,
            _pool: pool,
            clickhouse,
            staleness_engine,
            pages_manager,
            metrics,
            concurrency_limit: Arc::new(Semaphore::new(max_concurrency)),
        }
    }

    /// Start processing loop from a receiver channel.
    pub async fn start(self: Arc<Self>, mut receiver: mpsc::Receiver<UserActivity>) {
        info!("Activity Processor started. Ready for event stream.");
        
        let mut join_set = JoinSet::new();

        while let Some(activity) = receiver.recv().await {
            let processor = self.clone();
            
            // Limit concurrency
            let permit = match self.concurrency_limit.clone().acquire_owned().await {
                Ok(p) => p,
                Err(_) => break,
            };

            join_set.spawn(async move {
                let _permit = permit;
                if let Err(e) = processor.process_single(activity).await {
                    error!(error = %e, "Failed to process user activity");
                }
            });

            // Cleanup completed tasks
            while join_set.try_join_next().is_some() {}
        }
    }

    /// Process a single activity through all required sinks.
    async fn process_single(&self, activity: UserActivity) -> anyhow::Result<()> {
        let start = std::time::Instant::now();

        // 1. Sink to Postgres (Interaction Repo)
        self.interaction_repo.record_interaction(
            activity.user_id(),
            match activity {
                UserActivity::Playback { item_id, .. } | UserActivity::Reaction { item_id, .. } | UserActivity::Click { item_id, .. } | UserActivity::Impression { item_id, .. } => item_id,
                _ => 0,
            },
            activity.kind(),
            activity.scenario_slug().unwrap_or("unknown"),
            match activity {
                UserActivity::Playback { watch_percentage, .. } => watch_percentage,
                _ => 1.0,
            },
        ).await?;

        // 2. Sink to ClickHouse (for Analytics & Sidecar)
        if let Some(ref ch) = self.clickhouse {
            let ch_row = ClickHouseInteraction {
                user_id: activity.user_id(),
                item_id: match activity {
                    UserActivity::Playback { item_id, .. } | UserActivity::Reaction { item_id, .. } | UserActivity::Click { item_id, .. } | UserActivity::Impression { item_id, .. } => item_id,
                    _ => 0,
                },
                interaction_type: activity.kind().to_string(),
                scenario_slug: activity.scenario_slug().unwrap_or("unknown").to_string(),
                weight: 1.0,
                watch_duration_seconds: match activity {
                    UserActivity::Playback { watch_duration_seconds, .. } => watch_duration_seconds,
                    _ => 0,
                },
                created_at: chrono::Utc::now().timestamp() as u64,
            };
            
            let mut insert = ch.insert::<ClickHouseInteraction>("user_interactions").await?;
            insert.write(&ch_row).await?;
            insert.end().await?;
        }

        // 3. Notify Staleness Engine (Real-time cache invalidation)
        let event = match activity {
            UserActivity::Playback { user_id, item_id, watch_percentage, .. } => Some(UserEvent::WatchEvent { user_id, item_id, completion_rate: watch_percentage }),
            UserActivity::Reaction { user_id, item_id, ref reaction_type, .. } => Some(UserEvent::ExplicitFeedback { user_id, item_id, rating: if reaction_type == "like" { 1.0 } else { -1.0 } }),
            _ => None,
        };

        if let Some(ev) = event {
            self.staleness_engine.process_event(&ev).await?;
        }

        // 4. Notify Pages Manager (Real-time SDUI updates if needed)
        self.pages_manager.handle_activity(&activity).await?;

        self.metrics.record_ingestion_success(activity.kind(), start.elapsed()).await;
        Ok(())
    }
}
