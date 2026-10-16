//! Metered admission: per-capability concurrency limits and token-bucket
//! rate quotas.
//!
//! The [`ConcurrencyGuard`] tracks in-flight dispatches per capability and
//! enforces a configurable max-parallel cap. The [`TokenBucket`] is a
//! per-provider rate limiter using caller-supplied millisecond time.
//!
//! Both are deterministic and host-agnostic (zero I/O, zero deps) — the
//! caller drives the clock.

use std::collections::BTreeMap;
use std::fmt;

/// Tracks in-flight dispatches per capability, enforcing a max-parallel cap.
#[derive(Debug, Default)]
pub struct ConcurrencyGuard {
    /// Configured max-parallel per capability.
    caps: BTreeMap<String, usize>,
    /// Current in-flight count per capability.
    in_flight: BTreeMap<String, usize>,
}

/// Error returned when a capability is at its concurrency cap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConcurrencyExceeded {
    /// The capability that hit its cap.
    pub capability: String,
    /// Current in-flight count for the capability.
    pub in_flight: usize,
    /// The configured max-parallel cap.
    pub cap: usize,
}

impl fmt::Display for ConcurrencyExceeded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "concurrency cap exceeded for {}: {}/{} in flight",
            self.capability, self.in_flight, self.cap
        )
    }
}

impl std::error::Error for ConcurrencyExceeded {}

impl ConcurrencyGuard {
    /// Create an empty concurrency guard.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the max-parallel cap for a capability.
    pub fn set_cap(&mut self, capability: impl Into<String>, cap: usize) {
        self.caps.insert(capability.into(), cap);
    }

    /// Get the configured cap for a capability (default 0 = unlimited).
    #[must_use]
    pub fn cap(&self, capability: &str) -> usize {
        *self.caps.get(capability).unwrap_or(&0)
    }

    /// Current in-flight count for a capability.
    #[must_use]
    pub fn in_flight(&self, capability: &str) -> usize {
        *self.in_flight.get(capability).unwrap_or(&0)
    }

    /// Try to acquire a slot for a capability. Returns the new in-flight
    /// count, or an error if at cap.
    ///
    /// # Errors
    /// Returns [`ConcurrencyExceeded`] if the capability is at its cap.
    pub fn acquire(&mut self, capability: &str) -> Result<usize, ConcurrencyExceeded> {
        let cap = self.cap(capability);
        let current = self.in_flight(capability);
        if cap > 0 && current >= cap {
            return Err(ConcurrencyExceeded {
                capability: capability.into(),
                in_flight: current,
                cap,
            });
        }
        let new = current + 1;
        self.in_flight.insert(capability.into(), new);
        Ok(new)
    }

    /// Release a slot for a capability. Returns the new in-flight count.
    pub fn release(&mut self, capability: &str) -> usize {
        let current = self.in_flight(capability);
        if current == 0 {
            return 0; // underflow guard — idempotent release
        }
        let new = current - 1;
        self.in_flight.insert(capability.into(), new);
        new
    }
}

pub use ep_domain::{is_vertex_provider, RateLimited, TokenBucket, VertexQuotaPreset};

#[cfg(test)]
mod tests {
    use super::*;
    use ep_domain::Tick;

    // --- ConcurrencyGuard ---

    #[test]
    fn acquire_and_release() {
        let mut guard = ConcurrencyGuard::new();
        guard.set_cap("build", 2);

        assert_eq!(guard.acquire("build").unwrap(), 1);
        assert_eq!(guard.acquire("build").unwrap(), 2);
        assert_eq!(guard.in_flight("build"), 2);
    }

    #[test]
    fn acquire_at_cap_errors() {
        let mut guard = ConcurrencyGuard::new();
        guard.set_cap("build", 1);

        guard.acquire("build").unwrap();
        let err = guard.acquire("build").unwrap_err();
        assert_eq!(err.cap, 1);
        assert_eq!(err.in_flight, 1);
    }

    #[test]
    fn release_decrements_count() {
        let mut guard = ConcurrencyGuard::new();
        guard.set_cap("build", 3);

        guard.acquire("build").unwrap();
        guard.acquire("build").unwrap();
        assert_eq!(guard.in_flight("build"), 2);

        guard.release("build");
        assert_eq!(guard.in_flight("build"), 1);
    }

    #[test]
    fn release_below_zero_is_noop() {
        let mut guard = ConcurrencyGuard::new();
        assert_eq!(guard.release("build"), 0);
    }

    #[test]
    fn cap_zero_means_unlimited() {
        let mut guard = ConcurrencyGuard::new();
        // No cap set → cap is 0 → unlimited
        for _ in 0..100 {
            guard.acquire("build").unwrap();
        }
        assert_eq!(guard.in_flight("build"), 100);
    }

    #[test]
    fn separate_capabilities_are_independent() {
        let mut guard = ConcurrencyGuard::new();
        guard.set_cap("build", 1);
        guard.set_cap("review", 1);

        guard.acquire("build").unwrap();
        // "review" is independent — not blocked by "build"
        guard.acquire("review").unwrap();
        assert_eq!(guard.in_flight("build"), 1);
        assert_eq!(guard.in_flight("review"), 1);

        // "build" is at cap now
        assert!(guard.acquire("build").is_err());
        // "review" is also at cap
        assert!(guard.acquire("review").is_err());
    }

    // --- TokenBucket (integer u64 tokens & ticks) ---

    #[test]
    fn consume_succeeds_with_enough_tokens() {
        let mut bucket = TokenBucket::from_ticks(10, 1, 0);
        assert!(bucket.try_consume(5, 0).is_ok());
        assert_eq!(bucket.tokens(0), 5);
    }

    #[test]
    fn consume_fails_with_insufficient_tokens() {
        let mut bucket = TokenBucket::from_ticks(5, 1, 0);
        // Consume all 5
        bucket.try_consume(5, 0).unwrap();
        // Next consume fails
        let err = bucket.try_consume(1, 0).unwrap_err();
        assert_eq!(err.deficit_tokens, 1);
        assert_eq!(err.retry_after_ticks, 1);
        assert_eq!(err.retry_after_ms, 1);
    }

    #[test]
    fn tokens_refill_over_time() {
        let mut bucket = TokenBucket::from_ticks(10, 1, 0);
        bucket.try_consume(10, 0).unwrap();
        assert_eq!(bucket.tokens(0), 0);

        // After 5 ticks → 5 tokens refilled
        assert_eq!(bucket.tokens(5), 5);

        // After 10 ticks → 10 tokens (capped at capacity)
        assert_eq!(bucket.tokens(10), 10);
    }

    #[test]
    fn refill_caps_at_capacity() {
        let mut bucket = TokenBucket::from_ticks(5, 10, 0);
        bucket.try_consume(5, 0).unwrap();
        // After a very long time, should still be capped at 5
        assert_eq!(bucket.tokens(999_999), 5);
    }

    #[test]
    fn retry_after_calculation() {
        let mut bucket = TokenBucket::with_initial(10, 0, 2, Tick(0));
        let err = bucket.try_consume(10, 0).unwrap_err();
        // 10 deficit tokens / 2 per tick = 5 ticks
        assert_eq!(err.deficit_tokens, 10);
        assert_eq!(err.retry_after_ticks, 5);
        assert_eq!(err.retry_after_ms, 5);
    }

    #[test]
    fn recovery_after_token_refill() {
        let mut bucket = TokenBucket::from_ticks(1, 1, 0);
        bucket.try_consume(1, 0).unwrap();
        // Failed — no tokens
        assert!(bucket.try_consume(1, 0).is_err());
        // After 1 tick → 1 token refilled → success
        assert!(bucket.try_consume(1, 1).is_ok());
    }
}
