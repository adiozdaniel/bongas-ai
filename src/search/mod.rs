//! Phase 1.1: Embedded Search Pillar
//!
//! Organized into functional sub-modules for schema definition, 
//! index management, and linguistic analysis.

pub mod schema;
pub mod manager;
pub mod analyzer;

pub use schema::SearchSchema;
pub use manager::EmbeddedSearchManager;
pub use analyzer::sheng_analyzer;
