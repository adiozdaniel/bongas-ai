use std::sync::Arc;
pub use crate::engine::governance::orchestration::manager::service::PagesManager;
pub use crate::engine::governance::factory::scenarios_manager::service::ScenariosManager;
pub use crate::engine::governance::factory::scenario_factory::service::ScenarioFactory;

/// 🔐 THE BACKSTAGE: Administrative governance.
pub struct GovernancePillar {
    pub orchestration: Arc<PagesManager>,
    pub scenarios: Arc<ScenariosManager>,
    pub scenario_factory: Arc<ScenarioFactory>,
}

impl GovernancePillar {
    pub fn new(
        orchestration: Arc<PagesManager>,
        scenarios: Arc<ScenariosManager>,
        scenario_factory: Arc<ScenarioFactory>,
    ) -> Self {
        Self { orchestration, scenarios, scenario_factory }
    }
}
