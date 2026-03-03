//! Governance Pillar: Administrative and structural orchestration.

pub mod orchestration;
pub mod strategy;
pub mod factory;

use std::sync::Arc;
pub use orchestration::PagesManager;
pub use factory::scenarios_manager::ScenariosManager;
pub use factory::scenario_factory::ScenarioFactory;

/// 🔐 THE BACKSTAGE: Administrative governance.
pub struct GovernancePillar {
    pub orchestration: Arc<PagesManager>,
    pub scenarios: Arc<ScenariosManager>,
}

impl GovernancePillar {
    pub fn new(
        orchestration: Arc<PagesManager>,
        scenarios: Arc<ScenariosManager>,
    ) -> Self {
        Self { orchestration, scenarios }
    }
}
