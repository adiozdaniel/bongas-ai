//! Database layer — resilient storage, connection pooling, and repositories.

pub mod metrics;
pub mod models;
pub mod pool;
pub mod repositories;

// ─── Re-exports ─────────────────────────────────────────────────────────────

pub use metrics::{DatabaseMetrics, DatabaseMetricsSnapshot};
pub use pool::{PoolStats, ResilientPool, ResilientPoolConfig};
