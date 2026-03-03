//! V1 API — composes domain sub-routers into a single versioned router.
//! The Bongas-AI Symphony: Strict separation between Stage (Public) and Backstage (Admin).

pub mod stage;
pub mod backstage;
pub mod pulse;
pub mod router;

pub use router::routes;
