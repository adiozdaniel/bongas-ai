  pub mod rate_limit;
  pub mod error_handling;
  pub mod metrics;

  // Netflix-grade resilience middleware
  pub mod resilience;
  pub mod bulkhead;

  // Re-export key types
  pub use resilience::{ResilienceMiddleware, ResilienceConfig};
  pub use bulkhead::{BulkheadMiddleware, BulkheadConfig};
  pub use error_handling::{error_handling_middleware, EnhancedErrorMiddleware, validation_error_middleware};
  pub use metrics::{EndpointMetrics, DurationTracker, MetricsCollector};
  pub use rate_limit::{RateLimiter, RateLimitStatus};
