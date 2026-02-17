  pub mod rate_limit;
  pub mod error_handling;
  pub mod metrics;
  pub mod unified_error;
  pub mod platform_security;

  // Netflix-grade resilience middleware
  pub mod resilience;
  pub mod bulkhead;

  // Re-export key types
  pub use resilience::{ResilienceMiddleware, ResilienceConfig};
  pub use bulkhead::{BulkheadMiddleware, BulkheadConfig};
  pub use unified_error::unified_error_middleware;
  pub use metrics::{EndpointMetrics, DurationTracker, MetricsCollector};
  pub use rate_limit::{RateLimiter, RateLimitStatus};
