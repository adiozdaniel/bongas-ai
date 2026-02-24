//! Resilient database connection pooling.

pub mod service;

pub use service::{PoolStats, ResilientPool, ResilientPoolConfig};
