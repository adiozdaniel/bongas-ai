//! API-specific middleware application.

pub mod service;
pub mod identity;

pub use service::apply_middleware;
