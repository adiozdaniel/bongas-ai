//! Lock-free time-bucketed rolling window for failure rate tracking.
//!
//! Implements a ring buffer of time slices using pure atomic operations.

pub mod models;
pub mod service;

pub use self::models::WindowSnapshot;
pub use self::service::RollingWindow;
