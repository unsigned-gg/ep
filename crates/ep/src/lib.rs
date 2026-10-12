//! ep (Escapement Scheduler & Event Reactor)
//!
//! Metered, deterministic dispatch engine for autonomous agent constellations.
//! Pure-Rust, zero-dependency core dispatch, capability matching, custody DAGs, and tick scheduling.

pub use ep_domain::*;
pub use escapement_core as core;
pub use blackwall_bridge as bridge;
