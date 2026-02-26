//! Netflix-grade middleware stack for resilience, observability, and security.

pub mod rate_limit;
pub mod error_handling;
pub mod metrics;
pub mod unified_error;
pub mod platform_security;
pub mod resilience;
pub mod bulkhead;

// Re-export key types for public API stability
pub use resilience::{ResilienceMiddleware, ResilienceMiddlewareConfig};
pub use bulkhead::*;
pub use unified_error::*;
pub use metrics::*;
pub use rate_limit::*;
pub use platform_security::*;
pub use error_handling::*;
