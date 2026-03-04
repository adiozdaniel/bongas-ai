//! API layer — thin composition of versioned routes, middleware, and shared state.

pub mod v1;
pub mod models;
pub mod middleware;
pub mod router;

// Re-exports for external consumption
pub use router::service::create_router;
pub use models::system::context::models::ContextParams;
pub use models::system::models::*;
pub use models::response::models::StandardResponse;
pub use models::recommendation::models::*;
pub use models::scenario::models::*;
pub use middleware::service::{apply_middleware, extract_request_id, extract_request_id_from_headers};
pub use middleware::identity::service::{IdentityContext, identity_middleware};
pub use middleware::adaptive_limiter::service::{ConnectionTracker, adaptive_limiter_middleware};
pub use v1::router::service::routes;
