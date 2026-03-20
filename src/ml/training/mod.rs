//! Training domain — orchestrates the learning lifecycle for the Bongas engine.

pub mod candle;
pub mod online;
pub mod pillar;

pub use pillar::service::TrainingPillar;
