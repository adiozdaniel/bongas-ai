//! Intelligence Pillar: AI, monitoring, and proactive coordination.

use std::sync::Arc;
use crate::engine::intelligence::ai::suggestions_manager::service::SuggestionsManager;
use crate::engine::intelligence::ai::hive_mind::service::HiveMindConnector;
use crate::engine::intelligence::monitoring::analytics_sidecar::service::AnalyticsSidecar;
use crate::engine::intelligence::monitoring::staleness_engine::service::StalenessEngine;
use crate::engine::intelligence::workers::workers_manager::service::WorkersManager;
use crate::engine::intelligence::ai::simulator::service::SafetySimulator;
use crate::engine::intelligence::identity::service::IdentityStitcher;

use crate::engine::intelligence::monitoring::analytics_sidecar::service::UserEvent;

pub struct IntelligencePillar {
    pub suggestions: Arc<SuggestionsManager>,
    pub hive_mind: Arc<HiveMindConnector>,
    pub monitoring: Arc<AnalyticsSidecar>,
    pub staleness: Arc<StalenessEngine>,
    pub workers: Arc<WorkersManager>,
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
    ) -> Self {
        Self {
            suggestions,
            hive_mind,
            monitoring,
            staleness,
            workers,
            simulator: Arc::new(SafetySimulator::new()),
            identity: Arc::new(IdentityStitcher::new()),
        }
    }

    /// Global telemetry entry point: Record an event asynchronously.
    pub async fn record_event(&self, event: UserEvent) {
        self.monitoring.record_event(event).await;
    }
}
