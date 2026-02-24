//! API layer — thin composition of versioned routes, middleware, and shared state.

pub mod v1;
pub mod models;
pub mod middleware;
pub mod router;

pub use router::create_router;
