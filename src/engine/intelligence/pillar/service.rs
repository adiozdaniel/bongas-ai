//! Intelligence Pillar: AI, monitoring, and proactive coordination.

use std::sync::Arc;
use crate::engine::intelligence::ai::suggestions_manager::service::SuggestionsManager;
use crate::engine::intelligence::ai::hive_mind::service::HiveMindConnector;
use crate::engine::intelligence::monitoring::analytics_sidecar::service::AnalyticsSidecar;
use crate::engine::intelligence::monitoring::staleness_engine::service::StalenessEngine;
use crate::engine::intelligence::workers::WorkersManager;
use crate::engine::intelligence::workers::fatigue_sync::service::FatigueSynchronizer;
use crate::engine::intelligence::ai::simulator::service::SafetySimulator;
use crate::engine::intelligence::identity::service::IdentityStitcher;

use crate::engine::intelligence::monitoring::analytics_sidecar::service::UserEvent;

pub struct IntelligencePillar {
    pub suggestions: Arc<SuggestionsManager>,
    pub hive_mind: Arc<HiveMindConnector>,
    pub monitoring: Arc<AnalyticsSidecar>,
    pub staleness: Arc<StalenessEngine>,
    pub workers: Arc<WorkersManager>,
    pub fatigue_sync: Arc<FatigueSynchronizer>,
    pub simulator: Arc<SafetySimulator>,
    pub identity: Arc<IdentityStitcher>,
}

impl IntelligencePillar {
    pub fn new(
        suggestions: Arc<SuggestionsManager>,
        hive_mind: Arc<HiveMindConnector>,
        monitoring: Arc<AnalyticsSidecar>,
        staleness: Arc<StalenessEngine>,
        workers: Arc<WorkersManager>,
        fatigue_sync: Arc<FatigueSynchronizer>,
    ) -> Self {
        Self {
            suggestions,
            hive_mind,
            monitoring,
            staleness,
            workers,
            fatigue_sync,
            simulator: Arc::new(SafetySimulator::new()),
            identity: Arc::new(IdentityStitcher::new()),
        }
    }

    /// Global telemetry entry point: Record an event asynchronously.
    pub async fn record_event(&self, event: UserEvent) {
        self.monitoring.record_event(event).await;
    }

    /// Provide public access to the ClickHouse client.
    pub fn clickhouse_client(&self) -> Option<clickhouse::Client> {
        self.monitoring.clickhouse_client()
    }

    /// Inject engine reference into sub-components.
    pub fn set_engine(&self, engine: std::sync::Weak<crate::engine::coordination::service::BongasEngine>) {
        self.hive_mind.set_engine(engine.clone());
        self.monitoring.set_engine(engine);
    }
}
