use std::sync::Arc;
pub use crate::engine::intelligence::ai::suggestions_manager::service::SuggestionsManager;
pub use crate::engine::intelligence::ai::hive_mind::service::HiveMindConnector;
pub use crate::engine::intelligence::monitoring::analytics_sidecar::service::AnalyticsSidecar;
pub use crate::engine::intelligence::monitoring::staleness_engine::service::StalenessEngine;
pub use crate::engine::intelligence::workers::workers_manager::service::WorkersManager;

/// 📈 THE PULSE: AI-driven insights and monitoring.
pub struct IntelligencePillar {
    pub suggestions: Arc<SuggestionsManager>,
    pub hive_mind: Arc<HiveMindConnector>,
    pub monitoring: Arc<AnalyticsSidecar>,
    pub staleness: Arc<StalenessEngine>,
    pub workers: Arc<WorkersManager>,
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
        }
    }
}
