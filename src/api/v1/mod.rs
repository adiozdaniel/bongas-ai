//! V1 API — composes domain sub-routers into a single versioned router.
//! The Bongas-AI Symphony: Strict separation between Stage (Public) and Backstage (Admin).

pub mod recommendations;
pub mod scenarios;
pub mod pages;
pub mod features;
pub mod health;
pub mod admin;
pub mod experiments;
pub mod router;

pub use router::routes;
