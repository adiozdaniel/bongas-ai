//! Workers Intelligence: Background maintenance and task orchestration.

pub mod manager;
pub mod tribe_orchestrator;
pub mod regional_pulse;
pub mod fatigue_sync;

pub use manager::WorkersManager;
