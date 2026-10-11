//! ep (Event Poll & Async Reactor)
//!
//! Minimalist Linux epoll, signalfd, and pidfd event reactor with bounded memory queues and deterministic tick scheduling.
//!
//! # Architecture
//! Anchored in the systems principles of **Carl Lerche**.

pub use ep_domain::*;

/// Initializes the `ep` systems engine.
pub fn init() -> anyhow::Result<EpState> {
    Ok(EpState::default())
}
