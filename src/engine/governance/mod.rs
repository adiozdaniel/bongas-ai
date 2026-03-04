//! Governance Pillar: Administrative and structural orchestration.

pub mod orchestration;
pub mod strategy;
pub mod factory;
pub mod pillar;

pub use orchestration::manager::service::PagesManager;
pub use factory::scenarios_manager::service::ScenariosManager;
pub use factory::scenario_factory::service::ScenarioFactory;
pub use pillar::service::GovernancePillar;
