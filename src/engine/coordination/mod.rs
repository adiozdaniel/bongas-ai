//! Coordination domain — orchestrates the three pillars of the Bongas engine.

pub mod service;
pub mod builder;

pub use service::BongasEngine;
pub use builder::DiscoverySymphony;
