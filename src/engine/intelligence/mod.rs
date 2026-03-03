//! Intelligence Pillar: AI-driven insights and self-optimization.

pub mod ai;
pub mod monitoring;
pub mod workers;

use std::sync::Arc;
pub use ai::suggestions_manager::SuggestionsManager;
pub use ai::hive_mind::HiveMindConnector;
pub use monitoring::analytics_sidecar::AnalyticsSidecar;
pub use monitoring::staleness_engine::StalenessEngine;
pub use workers::workers_manager::WorkersManager;

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
