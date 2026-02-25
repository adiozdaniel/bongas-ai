//! Security module with Netflix-grade resilience patterns.

pub mod manager;
pub mod license;
pub mod hardware;
pub mod anti_debug;
pub mod binary;
pub mod validator;

// Re-export key types for public API stability
pub use manager::SecurityManager;
pub use license::*;
pub use hardware::*;
pub use anti_debug::*;
pub use binary::*;
pub use validator::*;
