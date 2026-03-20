//! High-performance activity processing orchestration.

use std::sync::Arc;
use tokio::sync::{mpsc, Semaphore};
use tracing::{info, error};

use crate::db::ResilientPool;
use crate::db::InteractionRepository;
use crate::engine::intelligence::monitoring::staleness_engine::service::{StalenessEngine, UserEvent as StalenessEvent};
use crate::engine::intelligence::monitoring::analytics_sidecar::service::UserEvent as AnalyticsEvent;
use crate::engine::intelligence::pillar::service::IntelligencePillar;
use crate::resilience::ResilienceMetricsCollector;

use crate::ingestion::UserActivity;
use tokio::task::JoinSet;
use crate::engine::governance::orchestration::manager::service::PagesManager;
use crate::db::repositories::interaction_repository::service::InteractionPayload;

use crate::cache::CacheManager;
use redis::AsyncCommands;

/// Processes activities from any source and routes them to DB + staleness engine + PagesManager.
pub struct ActivityProcessor {
    interaction_repo: Arc<InteractionRepository>,
    _pool: Arc<ResilientPool>,
    cache_manager: Arc<CacheManager>,
    intelligence: Arc<IntelligencePillar>,
    staleness_engine: Arc<StalenessEngine>,
    pages_manager: Arc<PagesManager>,
    metrics: Arc<ResilienceMetricsCollector>,
    concurrency_limit: Arc<Semaphore>,
}

impl ActivityProcessor {
    pub fn new(
        interaction_repo: Arc<InteractionRepository>,
        pool: Arc<ResilientPool>,
        cache_manager: Arc<CacheManager>,
        intelligence: Arc<IntelligencePillar>,
        staleness_engine: Arc<StalenessEngine>,
        pages_manager: Arc<PagesManager>,
        metrics: Arc<ResilienceMetricsCollector>,
    ) -> Self {
        Self {
            interaction_repo,
            _pool: pool,
            cache_manager,
            intelligence,
            staleness_engine,
            pages_manager,
            metrics,
            concurrency_limit: Arc::new(Semaphore::new(100)),
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

        // 1. Sink to Postgres (Interaction Repo) - RELATIONAL PERSISTENCE
        match activity {
            UserActivity::Playback { 
                user_id, ref profile_id, item_id, watch_duration_seconds, 
                watch_percentage, ref scenario_slug, ref visitor_id, ref device_hash, ref device_type, .. 
            } => {
                self.interaction_repo.create_implicit_rating(InteractionPayload {
                    user_id,
                    profile_id: profile_id.clone(),
                    item_id,
                    interaction_type: "implicit_rating".to_string(),
                    weight: watch_percentage,
                    watch_duration_seconds: Some(watch_duration_seconds),
                    scenario_slug: scenario_slug.as_deref().unwrap_or("unknown").to_string(),
                    visitor_id: visitor_id.clone(),
                    device_hash: device_hash.clone(),
                    device_type: device_type.clone(),
                }).await?;
            },
            _ => {
                self.interaction_repo.record_interaction(InteractionPayload {
                    user_id: activity.user_id(),
                    profile_id: activity.profile_id().map(|s| s.to_string()),
                    item_id: match activity {
                        UserActivity::Reaction { item_id, .. } | UserActivity::Click { item_id, .. } | UserActivity::Impression { item_id, .. } => item_id,
                        _ => 0,
                    },
                    interaction_type: activity.kind().to_string(),
                    scenario_slug: activity.scenario_slug().unwrap_or("unknown").to_string(),
                    weight: 1.0, // Default weight for non-playback
                    visitor_id: activity.visitor_id().map(|s| s.to_string()),
                    device_hash: activity.device_hash().map(|s| s.to_string()),
                    device_type: activity.device_type().map(|s| s.to_string()),
                    watch_duration_seconds: None,
                }).await?;
            }
        }

        // 2. Fetch tribe_id from Redis if profile_id is present
        let mut tribe_id = None;
        if let Some(pid) = activity.profile_id() {
            if let Some(mut conn) = self.cache_manager.l2_connection() {
                let key = format!("tribe_map:{}", pid);
                tribe_id = conn.get::<_, Option<i32>>(key).await.unwrap_or(None);
            }
        }

        // 3. Sink to ClickHouse (Asynchronous Buffered) - HIGH-VOLUME ANALYTICS
        let analytics_event = AnalyticsEvent {
            user_id: activity.user_id(),
            profile_id: activity.profile_id().map(|s| s.to_string()), 
            tribe_id,
            request_id: "ingested".to_string(),
            item_id: match activity {
                UserActivity::Playback { item_id, .. } | UserActivity::Reaction { item_id, .. } | UserActivity::Click { item_id, .. } | UserActivity::Impression { item_id, .. } => item_id,
                _ => 0,
            },
            interaction_type: activity.kind().to_string(),
            scenario_slug: activity.scenario_slug().map(|s| s.to_string()),
            device_type: activity.device_type().map(|s| s.to_string()),
            watch_duration_seconds: match activity {
                UserActivity::Playback { watch_duration_seconds, .. } => watch_duration_seconds,
                _ => 0,
            },
            created_at: chrono::Utc::now().timestamp() as u64,
        };
        
        self.intelligence.record_event(analytics_event).await;

        // 4. Notify Staleness Engine (Real-time cache invalidation)
        let event = match activity {
            UserActivity::Playback { user_id, ref profile_id, item_id, watch_percentage, .. } => {
                let pid = profile_id.clone().unwrap_or_else(|| "default".to_string());
                Some(StalenessEvent::WatchEvent { user_id, profile_id: pid, item_id, completion_rate: watch_percentage })
            },
            UserActivity::Reaction { user_id, ref profile_id, item_id, ref reaction_type, .. } => {
                let pid = profile_id.clone().unwrap_or_else(|| "default".to_string());
                Some(StalenessEvent::ExplicitFeedback { user_id, profile_id: pid, item_id, rating: if reaction_type == "like" { 1.0 } else { -1.0 } })
            },
            _ => None,
        };

        if let Some(e) = event {
            self.staleness_engine.on_user_event(&e).await?;
        }

        // 5. Update In-Memory Genesis Cache (Navigation Mesh)
        if let Some(pid) = activity.profile_id() {
            self.pages_manager.invalidate_cache(pid).await;
        }

        // 6. Reset Content Fatigue on Engagement
        if let Some(pid) = activity.profile_id() {
            let iid = match activity {
                UserActivity::Playback { item_id, .. } | UserActivity::Reaction { item_id, .. } | UserActivity::Click { item_id, .. } => Some(item_id),
                _ => None,
            };

            if let Some(item_id) = iid {
                let fatigue_sync = self.intelligence.fatigue_sync.clone();
                let pid_clone = pid.to_string();
                tokio::spawn(async move {
                    fatigue_sync.record_engagement(&pid_clone, item_id).await;
                });
            }
        }

        // 7. Notify Sequencing Intelligence (Pillar 4: Ghost Execution)
        self.intelligence.notify_activity(activity);

        self.metrics.record_interaction_processed(start.elapsed()).await;
        Ok(())
    }
}
