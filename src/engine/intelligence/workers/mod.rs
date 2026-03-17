//! Workers Intelligence: Background maintenance and task orchestration.

pub mod manager;
pub mod tribe_orchestrator;
pub mod regional_pulse;
pub mod fatigue_sync;
pub mod reasoning;
pub mod digest_worker;
pub mod search_sync;
pub mod signal_decay;
pub mod sovereign_sight;

pub use manager::WorkersManager;
