#![forbid(unsafe_code)]
//! Zero-I/O domain models for `ep` (Escapement Scheduler).
//!
//! # Invariants
//! This crate contains pure data structures, deterministic valve/tick/quota
//! state transitions, integer token-bucket rate metering, impulse ordering,
//! and stagnation detection. It performs zero I/O, network calls, floating-point
//! arithmetic, or operating-system operations.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Core state representation for `ep`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EpState {
    /// Protocol version.
    pub version: u32,
    /// Identifier.
    pub id: String,
}

impl Default for EpState {
    fn default() -> Self {
        Self {
            version: 1,
            id: String::from("initial"),
        }
    }
}

/// Monotonic discrete tick counter ($T \in \mathbb{N}_0$).
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct Tick(pub u64);

impl Tick {
    /// Origin tick (`0`).
    pub const ZERO: Self = Self(0);

    /// Construct a [`Tick`] from a raw `u64`.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the underlying `u64` tick value.
    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    /// Advance the tick by `delta`, saturating at `u64::MAX`.
    #[must_use]
    pub const fn advance(self, delta: u64) -> Self {
        Self(self.0.saturating_add(delta))
    }

    /// Compute elapsed ticks since `earlier` (`0` if `self <= earlier`).
    #[must_use]
    pub const fn elapsed_since(self, earlier: Self) -> u64 {
        self.0.saturating_sub(earlier.0)
    }
}

impl fmt::Display for Tick {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u64> for Tick {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<Tick> for u64 {
    fn from(tick: Tick) -> Self {
        tick.0
    }
}

/// Monotonic intra-tick sequence number for deterministic tie-breaking.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct ImpulseSeq(pub u64);

impl ImpulseSeq {
    /// Initial sequence number (`0`).
    pub const ZERO: Self = Self(0);

    /// Construct an [`ImpulseSeq`] from a raw `u64`.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the underlying `u64` sequence value.
    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    /// Return the next sequence number, saturating at `u64::MAX`.
    #[must_use]
    pub const fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

impl fmt::Display for ImpulseSeq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Composite `(Tick, ImpulseSeq)` stamp providing total deterministic ordering
/// across ticks and within the same tick.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct ImpulseStamp {
    /// Discrete tick at which the impulse occurred.
    pub tick: Tick,
    /// Monotonic sequence number within `tick`.
    pub seq: ImpulseSeq,
}

impl ImpulseStamp {
    /// Construct a new [`ImpulseStamp`].
    #[must_use]
    pub const fn new(tick: Tick, seq: ImpulseSeq) -> Self {
        Self { tick, seq }
    }
}

/// Periodic tick cadence defining how often an escapement impulse fires.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Cadence {
    /// Number of ticks between consecutive impulses (must be `>= 1`).
    pub interval_ticks: u64,
}

impl Cadence {
    /// Construct a validated [`Cadence`].
    ///
    /// # Errors
    /// Returns [`EpError::ZeroCadenceInterval`] if `interval_ticks == 0`.
    pub const fn new(interval_ticks: u64) -> Result<Self, EpError> {
        if interval_ticks == 0 {
            Err(EpError::ZeroCadenceInterval)
        } else {
            Ok(Self { interval_ticks })
        }
    }

    /// Returns `true` when `tick` falls on an exact multiple of `interval_ticks`.
    #[must_use]
    pub const fn is_due(self, tick: Tick) -> bool {
        self.interval_ticks > 0 && (tick.0 % self.interval_ticks) == 0
    }

    /// Compute the next tick strictly after `current` that aligns with `interval_ticks`.
    #[must_use]
    pub const fn next_tick(self, current: Tick) -> Tick {
        if self.interval_ticks == 0 {
            return current;
        }
        let rem = current.0 % self.interval_ticks;
        let delta = self.interval_ticks - rem;
        Tick(current.0.saturating_add(delta))
    }
}

/// Error returned when a [`TokenBucket`] has insufficient integer tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RateLimited {
    /// Token deficit (`requested - available`).
    pub deficit_tokens: u64,
    /// Monotonic ticks the caller must wait before retrying (`ceil(deficit / refill_per_tick)`).
    pub retry_after_ticks: u64,
    /// Alias of `retry_after_ticks` for millisecond-tick callers in `escapement-core::metering`.
    pub retry_after_ms: u64,
}

impl fmt::Display for RateLimited {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "rate limited: deficit {} tokens, retry after {} ticks",
            self.deficit_tokens, self.retry_after_ticks
        )
    }
}

impl std::error::Error for RateLimited {}

/// Domain errors for `ep-domain` invariants.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(rename_all = "snake_case")]
pub enum EpError {
    /// Token bucket capacity must be greater than zero.
    #[error("token bucket capacity must be greater than zero")]
    ZeroCapacity,
    /// Cadence interval must be greater than zero ticks.
    #[error("cadence interval_ticks must be greater than zero")]
    ZeroCadenceInterval,
    /// Token bucket rate limit exceeded.
    #[error("{0}")]
    RateLimited(#[from] RateLimited),
    /// Capability slug violates `[a-z0-9_-]{1,64}` invariant.
    #[error(
        "invalid capability slug '{0}': must be 1..=64 ASCII lowercase alphanumeric, '-', or '_'"
    )]
    InvalidCapabilitySlug(String),
    /// Pool delivery reference violates Cortex stream_seq or pbjson_types::Struct invariants.
    #[error("invalid pool delivery refs: {0}")]
    InvalidPoolDeliveryRefs(String),
    /// Pool result reference violates Stage A mesh-pool contract invariants.
    #[error("invalid pool result refs: {0}")]
    InvalidPoolResultRefs(String),
}

/// Type alias for domain errors used by downstream workspace crates.
pub type DomainError = EpError;

/// Operating state of the escapement release valve.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValveState {
    /// Valve is sealed; no new impulses may be released.
    #[default]
    Closed,
    /// Valve is open; impulses are metered against [`TickQuota`] or [`TokenBucket`].
    Open,
    /// Valve is draining in-flight work before closing; new impulses are held.
    Draining,
}

impl ValveState {
    /// Returns `true` only when the valve is [`ValveState::Open`].
    #[must_use]
    pub const fn accepts_impulses(self) -> bool {
        matches!(self, Self::Open)
    }
}

/// Deterministic tick quota budget for an escapement lane or provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TickQuota {
    /// Maximum ticks that can accumulate in the budget.
    pub capacity: u64,
    /// Currently available ticks ready for release.
    pub available: u64,
    /// Maximum ticks that a single impulse is allowed to consume (`0` = bounded only by `capacity`).
    pub max_per_impulse: u64,
}

impl TickQuota {
    /// Construct a [`TickQuota`], clamping `available` to `capacity`.
    #[must_use]
    pub const fn new(capacity: u64, available: u64, max_per_impulse: u64) -> Self {
        let clamped_available = if available > capacity {
            capacity
        } else {
            available
        };
        Self {
            capacity,
            available: clamped_available,
            max_per_impulse,
        }
    }

    /// Construct a full [`TickQuota`] with `available == capacity` and no per-impulse cap beyond `capacity`.
    #[must_use]
    pub const fn full(capacity: u64) -> Self {
        Self {
            capacity,
            available: capacity,
            max_per_impulse: capacity,
        }
    }

    /// Refill the quota by `ticks`, saturating at `capacity`.
    pub fn refill(&mut self, ticks: u64) {
        self.available = self.available.saturating_add(ticks).min(self.capacity);
    }
}

/// Reason why an escapement impulse was held rather than permitted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HoldReason {
    /// The escapement valve is [`ValveState::Closed`].
    ValveClosed,
    /// The escapement valve is [`ValveState::Draining`].
    ValveDraining,
    /// An impulse requested zero ticks.
    ZeroImpulseRequested,
    /// Requested ticks exceed the per-impulse cap.
    ExceedsMaxPerImpulse {
        requested: u64,
        max_per_impulse: u64,
    },
    /// Requested ticks exceed the currently available quota.
    QuotaExhausted { requested: u64, available: u64 },
}

impl fmt::Display for HoldReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ValveClosed => f.write_str("valve is closed"),
            Self::ValveDraining => f.write_str("valve is draining"),
            Self::ZeroImpulseRequested => f.write_str("impulse must request at least 1 tick"),
            Self::ExceedsMaxPerImpulse {
                requested,
                max_per_impulse,
            } => write!(
                f,
                "requested {requested} ticks exceeds max_per_impulse {max_per_impulse}"
            ),
            Self::QuotaExhausted {
                requested,
                available,
            } => write!(
                f,
                "quota exhausted: requested {requested} ticks, only {available} available"
            ),
        }
    }
}

/// Pure admission decision for an escapement impulse or token-bucket valve check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseDecision {
    /// Impulse is permitted; `remaining` is the post-release quota balance.
    Permit { remaining: u64 },
    /// Impulse is held back with a structured [`HoldReason`].
    Hold { reason: HoldReason },
}

impl ReleaseDecision {
    /// Returns `true` if the decision is [`ReleaseDecision::Permit`].
    #[must_use]
    pub const fn is_permitted(&self) -> bool {
        matches!(self, Self::Permit { .. })
    }

    /// Returns `true` if the decision is [`ReleaseDecision::Hold`].
    #[must_use]
    pub const fn is_held(&self) -> bool {
        matches!(self, Self::Hold { .. })
    }
}

/// Canonical alias for [`ReleaseDecision`] used across valve and token-bucket APIs.
pub type ValveDecision = ReleaseDecision;

/// Pure decision function evaluating whether an impulse requesting `requested`
/// ticks may pass through the escapement given `state` and `quota`.
#[must_use]
pub fn evaluate_impulse(state: ValveState, quota: &TickQuota, requested: u64) -> ReleaseDecision {
    match state {
        ValveState::Closed => {
            return ReleaseDecision::Hold {
                reason: HoldReason::ValveClosed,
            }
        }
        ValveState::Draining => {
            return ReleaseDecision::Hold {
                reason: HoldReason::ValveDraining,
            }
        }
        ValveState::Open => {}
    }

    if requested == 0 {
        return ReleaseDecision::Hold {
            reason: HoldReason::ZeroImpulseRequested,
        };
    }

    if quota.max_per_impulse > 0 && requested > quota.max_per_impulse {
        return ReleaseDecision::Hold {
            reason: HoldReason::ExceedsMaxPerImpulse {
                requested,
                max_per_impulse: quota.max_per_impulse,
            },
        };
    }

    if requested > quota.available {
        return ReleaseDecision::Hold {
            reason: HoldReason::QuotaExhausted {
                requested,
                available: quota.available,
            },
        };
    }

    ReleaseDecision::Permit {
        remaining: quota.available - requested,
    }
}

/// Monotonic integer token-bucket rate limiter $(R, B)$ refilled on [`Tick`] boundaries.
///
/// Per `SPEC-0001` Requirement 7, all token accounting uses pure `u64` integer
/// arithmetic without floating-point operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenBucket {
    /// Maximum integer tokens the bucket can hold ($B$).
    pub capacity: u64,
    /// Currently available integer tokens.
    pub available: u64,
    /// Integer tokens added per elapsed tick ($R$).
    pub refill_per_tick: u64,
    /// Last monotonic tick at which the bucket was refilled.
    pub last_tick: Tick,
}

impl TokenBucket {
    /// Construct a new [`TokenBucket`] starting full at `start_tick`.
    ///
    /// # Errors
    /// Returns [`EpError::ZeroCapacity`] if `capacity == 0`.
    pub const fn new(
        capacity: u64,
        refill_per_tick: u64,
        start_tick: Tick,
    ) -> Result<Self, EpError> {
        if capacity == 0 {
            return Err(EpError::ZeroCapacity);
        }
        Ok(Self {
            capacity,
            available: capacity,
            refill_per_tick,
            last_tick: start_tick,
        })
    }

    /// Construct a [`TokenBucket`] with an explicit initial `available` balance
    /// (clamped to `capacity`). Allows zero-capacity buckets for testing.
    #[must_use]
    pub const fn with_initial(
        capacity: u64,
        available: u64,
        refill_per_tick: u64,
        start_tick: Tick,
    ) -> Self {
        let clamped_available = if available > capacity {
            capacity
        } else {
            available
        };
        Self {
            capacity,
            available: clamped_available,
            refill_per_tick,
            last_tick: start_tick,
        }
    }

    /// Construct a full [`TokenBucket`] from raw `u64` tick coordinates.
    #[must_use]
    pub const fn from_ticks(capacity: u64, refill_per_tick: u64, start_tick: u64) -> Self {
        Self::with_initial(capacity, capacity, refill_per_tick, Tick(start_tick))
    }

    /// Refill the bucket monotonically up to `now` using pure integer arithmetic.
    ///
    /// If `now <= self.last_tick`, this is a no-op (monotonic clock invariant).
    pub fn refill_to(&mut self, now: Tick) {
        if now.0 > self.last_tick.0 {
            let elapsed = now.0 - self.last_tick.0;
            let added = elapsed.saturating_mul(self.refill_per_tick);
            self.available = self.available.saturating_add(added).min(self.capacity);
            self.last_tick = now;
        }
    }

    /// Refill to `now` and attempt to acquire `cost` integer tokens, returning a [`ValveDecision`].
    pub fn try_acquire(&mut self, now: Tick, cost: u64) -> ValveDecision {
        self.refill_to(now);
        if cost == 0 {
            return ValveDecision::Hold {
                reason: HoldReason::ZeroImpulseRequested,
            };
        }
        if self.available >= cost {
            self.available -= cost;
            ValveDecision::Permit {
                remaining: self.available,
            }
        } else {
            ValveDecision::Hold {
                reason: HoldReason::QuotaExhausted {
                    requested: cost,
                    available: self.available,
                },
            }
        }
    }

    /// Refill to `now`, evaluate against `state` and `max_per_impulse`, and deduct `cost` if permitted.
    pub fn try_acquire_with_valve(
        &mut self,
        state: ValveState,
        now: Tick,
        max_per_impulse: u64,
        cost: u64,
    ) -> ValveDecision {
        self.refill_to(now);
        let quota = self.to_tick_quota(max_per_impulse);
        let decision = evaluate_impulse(state, &quota, cost);
        if let ValveDecision::Permit { remaining } = decision {
            self.available = remaining;
        }
        decision
    }

    /// Try to consume `cost` integer tokens at `now_tick`.
    ///
    /// # Errors
    /// Returns [`RateLimited`] with exact integer `deficit_tokens` and `retry_after_ticks`
    /// if the bucket has fewer than `cost` tokens.
    pub fn try_consume(&mut self, cost: u64, now_tick: u64) -> Result<(), RateLimited> {
        self.refill_to(Tick(now_tick));
        if self.available >= cost {
            self.available -= cost;
            Ok(())
        } else {
            let deficit_tokens = cost - self.available;
            let retry_after_ticks = if self.refill_per_tick > 0 {
                deficit_tokens.div_ceil(self.refill_per_tick)
            } else {
                u64::MAX
            };
            Err(RateLimited {
                deficit_tokens,
                retry_after_ticks,
                retry_after_ms: retry_after_ticks,
            })
        }
    }

    /// Refill at `now_tick` and return the current integer token balance.
    #[must_use]
    pub fn tokens(&mut self, now_tick: u64) -> u64 {
        self.refill_to(Tick(now_tick));
        self.available
    }

    /// Snapshot the bucket's current capacity and balance as a [`TickQuota`].
    #[must_use]
    pub const fn to_tick_quota(&self, max_per_impulse: u64) -> TickQuota {
        TickQuota::new(self.capacity, self.available, max_per_impulse)
    }

    /// Construct a [`TokenBucket`] with the default [`VertexQuotaPreset::ceres_default`] parameters.
    #[must_use]
    pub const fn vertex_default(start_tick: Tick) -> Self {
        VertexQuotaPreset::ceres_default().to_token_bucket(start_tick)
    }
}

/// Deterministic integer rate-limit and concurrency preset for Google Cloud Vertex AI (`google-vertex`).
///
/// Prevents burst HTTP 429 `RESOURCE_EXHAUSTED` across concurrent workstation and cluster lanes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VertexQuotaPreset {
    /// Token bucket burst capacity ($B$).
    pub burst_capacity: u64,
    /// Token bucket refill rate per tick ($R$).
    pub refill_per_tick: u64,
    /// Maximum concurrent in-flight Vertex AI streams per host/orchestrator.
    pub max_concurrent: usize,
    /// Base exponential backoff ticks on HTTP 429 `RESOURCE_EXHAUSTED`.
    pub base_429_backoff_ticks: u64,
    /// Maximum exponential backoff ticks on HTTP 429 `RESOURCE_EXHAUSTED`.
    pub max_429_backoff_ticks: u64,
}

impl VertexQuotaPreset {
    /// Conservative default preset for shared hosts (`ceres` / `unsigned-paas`):
    /// - `burst_capacity`: `15`
    /// - `refill_per_tick`: `1`
    /// - `max_concurrent`: `4`
    /// - `base_429_backoff_ticks`: `20` (2,000ms at 100ms ticks)
    /// - `max_429_backoff_ticks`: `300` (30,000ms at 100ms ticks)
    #[must_use]
    pub const fn ceres_default() -> Self {
        Self {
            burst_capacity: 15,
            refill_per_tick: 1,
            max_concurrent: 4,
            base_429_backoff_ticks: 20,
            max_429_backoff_ticks: 300,
        }
    }

    /// Construct a [`TokenBucket`] initialized from this Vertex preset at `start_tick`.
    #[must_use]
    pub const fn to_token_bucket(self, start_tick: Tick) -> TokenBucket {
        TokenBucket::with_initial(
            self.burst_capacity,
            self.burst_capacity,
            self.refill_per_tick,
            start_tick,
        )
    }

    /// Compute deterministic integer exponential backoff ticks for `consecutive_429s` (`>= 1`).
    #[must_use]
    pub const fn backoff_ticks_for_429(self, consecutive_429s: u32) -> u64 {
        if consecutive_429s == 0 {
            return 0;
        }
        let shift = if consecutive_429s > 6 {
            6
        } else {
            consecutive_429s - 1
        };
        let exp = self.base_429_backoff_ticks.saturating_mul(1_u64 << shift);
        if exp > self.max_429_backoff_ticks {
            self.max_429_backoff_ticks
        } else {
            exp
        }
    }
}

impl Default for VertexQuotaPreset {
    fn default() -> Self {
        Self::ceres_default()
    }
}

/// Returns `true` if `provider` identifies a Google Cloud Vertex AI endpoint
/// (`"vertex"`, `"google-vertex"`, `"vertex-ai"`).
#[must_use]
pub fn is_vertex_provider(provider: &str) -> bool {
    matches!(
        provider.trim(),
        "vertex" | "google-vertex" | "vertex-ai" | "VERTEX" | "GOOGLE-VERTEX" | "VERTEX-AI"
    )
}

/// Stateless stagnation classifications for iterative agent/scheduler loops.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StagnationPattern {
    /// Identical state hash repeated consecutively (`A -> A -> A`).
    Spinning,
    /// Periodic 2-cycle between distinct state hashes (`A -> B -> A -> B`).
    Oscillation,
    /// Zero progress delta across the observation window despite steps executing.
    NoDrift,
    /// Strictly positive but monotonically shrinking progress deltas.
    DiminishingReturns,
}

/// Lateral thinking persona injected by the scheduler to break a [`StagnationPattern`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LateralPersona {
    /// Challenges hidden assumptions when an agent repeats the same error (`Spinning`).
    Contrarian,
    /// Unblocks zero-delta stalls with a minimal concrete vertical slice (`NoDrift`).
    Hacker,
    /// Strips incidental complexity when progress deltas shrink (`DiminishingReturns`).
    Simplifier,
    /// Gathers missing specification or codebase evidence before mutating state.
    Researcher,
    /// Restructures coupled boundaries to break `A <-> B` oscillation cycles (`Oscillation`).
    Architect,
}

impl LateralPersona {
    /// Ordered ring of all lateral personas for deterministic attempt rotation.
    pub const ALL: [Self; 5] = [
        Self::Contrarian,
        Self::Hacker,
        Self::Simplifier,
        Self::Researcher,
        Self::Architect,
    ];
}

/// Pure deterministic selector mapping a [`StagnationPattern`] and 0-indexed `attempt`
/// counter to a [`LateralPersona`].
///
/// Primary mapping (`attempt == 0`):
/// - [`StagnationPattern::Spinning`] -> [`LateralPersona::Contrarian`]
/// - [`StagnationPattern::Oscillation`] -> [`LateralPersona::Architect`]
/// - [`StagnationPattern::NoDrift`] -> [`LateralPersona::Hacker`]
/// - [`StagnationPattern::DiminishingReturns`] -> [`LateralPersona::Simplifier`]
///
/// Subsequent attempts (`attempt > 0`) rotate deterministically around [`LateralPersona::ALL`].
#[must_use]
pub const fn select_lateral_persona(pattern: StagnationPattern, attempt: u32) -> LateralPersona {
    let base_idx: usize = match pattern {
        StagnationPattern::Spinning => 0,           // Contrarian
        StagnationPattern::NoDrift => 1,            // Hacker
        StagnationPattern::DiminishingReturns => 2, // Simplifier
        StagnationPattern::Oscillation => 4,        // Architect
    };
    let idx = (base_idx + (attempt as usize)) % LateralPersona::ALL.len();
    LateralPersona::ALL[idx]
}

/// Deterministic control directive emitted when evaluating loop progress or stagnation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlDirective {
    /// Progress is healthy; continue normal tick execution.
    Continue,
    /// Spinning stall detected; retry with perturbed parameters.
    Retry,
    /// Zero drift observed; wait for next cadence or external input.
    Wait,
    /// Unrecoverable oscillation cycle detected; cancel the current run.
    Cancel,
    /// Diminishing returns reached convergence; finalize and settle.
    Converge,
    /// Inject a lateral persona to break a detected [`StagnationPattern`].
    InjectPersona {
        /// Selected lateral persona for this stagnation attempt.
        persona: LateralPersona,
        /// Underlying stagnation pattern that triggered the persona switch.
        reason: StagnationPattern,
    },
}

/// Stateless detector classifying stagnation from recent state hashes and progress deltas.
///
/// Evaluation order:
/// 1. [`StagnationPattern::Spinning`] (`>= 3` identical trailing hashes) -> [`ControlDirective::Retry`]
/// 2. [`StagnationPattern::Oscillation`] (`>= 4` alternating `A, B, A, B` trailing hashes) -> [`ControlDirective::Cancel`]
/// 3. [`StagnationPattern::NoDrift`] (`>= 3` trailing zero progress deltas) -> [`ControlDirective::Wait`]
/// 4. [`StagnationPattern::DiminishingReturns`] (`>= 3` strictly decreasing positive deltas) -> [`ControlDirective::Converge`]
#[must_use]
pub fn detect_stagnation(
    history_hashes: &[u64],
    progress_deltas: &[u64],
) -> Option<(StagnationPattern, ControlDirective)> {
    let h_len = history_hashes.len();
    if h_len >= 3 {
        let a = history_hashes[h_len - 3];
        let b = history_hashes[h_len - 2];
        let c = history_hashes[h_len - 1];
        if a == b && b == c {
            return Some((StagnationPattern::Spinning, ControlDirective::Retry));
        }
    }

    if h_len >= 4 {
        let a0 = history_hashes[h_len - 4];
        let b0 = history_hashes[h_len - 3];
        let a1 = history_hashes[h_len - 2];
        let b1 = history_hashes[h_len - 1];
        if a0 == a1 && b0 == b1 && a0 != b0 {
            return Some((StagnationPattern::Oscillation, ControlDirective::Cancel));
        }
    }

    let d_len = progress_deltas.len();
    if d_len >= 3 {
        let d0 = progress_deltas[d_len - 3];
        let d1 = progress_deltas[d_len - 2];
        let d2 = progress_deltas[d_len - 1];
        if d0 == 0 && d1 == 0 && d2 == 0 {
            return Some((StagnationPattern::NoDrift, ControlDirective::Wait));
        }
        if d0 > d1 && d1 > d2 && d2 > 0 {
            return Some((
                StagnationPattern::DiminishingReturns,
                ControlDirective::Converge,
            ));
        }
    }

    None
}

/// Detect stagnation and emit a [`ControlDirective::InjectPersona`] directive using
/// [`select_lateral_persona`] for the given 0-indexed `attempt`.
#[must_use]
pub fn detect_stagnation_with_persona(
    history_hashes: &[u64],
    progress_deltas: &[u64],
    attempt: u32,
) -> Option<(StagnationPattern, ControlDirective)> {
    let (pattern, _) = detect_stagnation(history_hashes, progress_deltas)?;
    let persona = select_lateral_persona(pattern, attempt);
    Some((
        pattern,
        ControlDirective::InjectPersona {
            persona,
            reason: pattern,
        },
    ))
}

/// Maximum integer representable without precision loss in Cortex `pbjson_types::Struct` (`2^53`).
pub const MAX_SAFE_STRUCT_INT: u64 = 9_007_199_254_740_992;

/// Validated capability slug (`[a-z0-9_-]{1,64}`) mapping an Escapement task capability
/// to a Cortex Mesh Pool target (`workstreams/<slug>`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CapabilitySlug(String);

impl CapabilitySlug {
    /// Validate and construct a [`CapabilitySlug`].
    ///
    /// # Errors
    /// Returns [`EpError::InvalidCapabilitySlug`] if `slug` is empty, exceeds 64 bytes,
    /// or contains characters outside `[a-z0-9_-]`.
    pub fn new(slug: impl Into<String>) -> Result<Self, EpError> {
        let s = slug.into();
        if s.is_empty()
            || s.len() > 64
            || !s
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
        {
            return Err(EpError::InvalidCapabilitySlug(s));
        }
        Ok(Self(s))
    }

    /// Return the validated slug string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Format the Cortex Mesh Pool route target (`workstreams/<slug>`).
    #[must_use]
    pub fn workstream_target(&self) -> String {
        format!("workstreams/{}", self.0)
    }

    /// Format the Cortex firewall allowlist entry (`pool:<slug>`).
    #[must_use]
    pub fn pool_firewall_entry(&self) -> String {
        format!("pool:{}", self.0)
    }

    /// Format the Cortex mesh recipient address (`pool:<slug>`).
    #[must_use]
    pub fn pool_address(&self) -> String {
        format!("pool:{}", self.0)
    }
}

impl fmt::Display for CapabilitySlug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Custody execution mode for an admitted Escapement task (`SPEC-0003`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CustodyMode {
    /// Direct worker or `blackwall-bridge` custody (`POST /v1/runs` or `POST /tasks/claim`).
    DirectWorker {
        /// Assigned worker agent identifier.
        agent_id: String,
    },
    /// Cortex Mesh Pool custody (`POST /v1/mesh/messages` to `pool:<slug>`).
    Pool {
        /// Validated capability pool slug.
        pool: CapabilitySlug,
    },
}

/// Immutable Cortex Mesh Pool delivery references recorded on pool dispatch (`SPEC-0003`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PoolDeliveryRefs {
    /// Target capability pool slug.
    pub pool: CapabilitySlug,
    /// Monotonic stream sequence number (`1 <= stream_seq <= MAX_SAFE_STRUCT_INT`).
    pub stream_seq: u64,
    /// Escapement task identifier (`refs.task_id`).
    pub task_id: String,
    /// 1-indexed dispatch attempt counter (`>= 1`).
    pub attempt: u32,
    /// Optional W3C `traceparent` header propagated in `refs.traceparent`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub traceparent: Option<String>,
}

impl PoolDeliveryRefs {
    /// Validate and construct [`PoolDeliveryRefs`].
    ///
    /// # Errors
    /// Returns [`EpError::InvalidPoolDeliveryRefs`] if `task_id` is empty,
    /// `attempt == 0`, `stream_seq == 0`, or `stream_seq > MAX_SAFE_STRUCT_INT`.
    pub fn new(
        pool: CapabilitySlug,
        stream_seq: u64,
        task_id: impl Into<String>,
        attempt: u32,
        traceparent: Option<String>,
    ) -> Result<Self, EpError> {
        let task_id = task_id.into();
        if task_id.trim().is_empty() {
            return Err(EpError::InvalidPoolDeliveryRefs(String::from(
                "task_id must not be empty",
            )));
        }
        if attempt == 0 {
            return Err(EpError::InvalidPoolDeliveryRefs(String::from(
                "attempt must be >= 1",
            )));
        }
        if stream_seq == 0 {
            return Err(EpError::InvalidPoolDeliveryRefs(String::from(
                "stream_seq must be >= 1",
            )));
        }
        if stream_seq > MAX_SAFE_STRUCT_INT {
            return Err(EpError::InvalidPoolDeliveryRefs(format!(
                "stream_seq {stream_seq} exceeds MAX_SAFE_STRUCT_INT ({MAX_SAFE_STRUCT_INT})"
            )));
        }
        Ok(Self {
            pool,
            stream_seq,
            task_id,
            attempt,
            traceparent,
        })
    }
}

/// Stage A `mesh-pool` result outcome (`"ok"`, `"transient"`, `"permanent"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PoolResultOutcome {
    /// Job succeeded (`"ok"` or `"completed"`).
    Ok,
    /// Retryable failure (`"transient"`).
    Transient,
    /// Terminal non-retryable failure (`"permanent"` or `"failed"`).
    Permanent,
}

impl PoolResultOutcome {
    /// Parse a wire outcome string (`"ok"`, `"transient"`, `"permanent"`).
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim() {
            "ok" | "completed" => Some(Self::Ok),
            "transient" => Some(Self::Transient),
            "permanent" | "failed" => Some(Self::Permanent),
            _ => None,
        }
    }

    /// Return the canonical Stage A wire string.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Transient => "transient",
            Self::Permanent => "permanent",
        }
    }
}

/// Maximum byte length of `refs.error` in a `mesh-pool` Stage A result frame.
pub const MAX_POOL_ERROR_BYTES: usize = 1024;

/// Result frame references returned by a `mesh-pool` worker (`LaneMessageKind::Msg`)
/// to `service-escapement-pool`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PoolResultRefs {
    /// Echoed Cortex stream sequence number (`refs.stream_seq`).
    pub stream_seq: u64,
    /// Escapement task identifier (`refs.task_id`).
    pub task_id: String,
    /// Attempt counter echoed from the inbound delivery (`>= 1`).
    pub attempt: u32,
    /// Worker outcome (`Ok`, `Transient`, `Permanent`).
    pub outcome: PoolResultOutcome,
    /// Optional URL-redacted error message (capped at 1024 bytes by `mesh-pool`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Optional W3C `traceparent` echoed verbatim by `mesh-pool`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub traceparent: Option<String>,
}

impl PoolResultRefs {
    /// Validate and construct [`PoolResultRefs`].
    ///
    /// # Errors
    /// Returns [`EpError::InvalidPoolResultRefs`] if `stream_seq == 0`,
    /// `stream_seq > MAX_SAFE_STRUCT_INT`, `task_id` is empty, `attempt == 0`,
    /// or `error` exceeds [`MAX_POOL_ERROR_BYTES`].
    pub fn new(
        stream_seq: u64,
        task_id: impl Into<String>,
        attempt: u32,
        outcome: PoolResultOutcome,
        error: Option<String>,
        traceparent: Option<String>,
    ) -> Result<Self, EpError> {
        let task_id = task_id.into();
        if task_id.trim().is_empty() {
            return Err(EpError::InvalidPoolResultRefs(String::from(
                "task_id must not be empty",
            )));
        }
        if attempt == 0 {
            return Err(EpError::InvalidPoolResultRefs(String::from(
                "attempt must be >= 1",
            )));
        }
        if stream_seq == 0 || stream_seq > MAX_SAFE_STRUCT_INT {
            return Err(EpError::InvalidPoolResultRefs(format!(
                "stream_seq {stream_seq} out of bounds 1..={MAX_SAFE_STRUCT_INT}"
            )));
        }
        if let Some(ref err) = error {
            if err.len() > MAX_POOL_ERROR_BYTES {
                return Err(EpError::InvalidPoolResultRefs(format!(
                    "error length {} exceeds MAX_POOL_ERROR_BYTES ({MAX_POOL_ERROR_BYTES})",
                    err.len()
                )));
            }
        }
        Ok(Self {
            stream_seq,
            task_id,
            attempt,
            outcome,
            error,
            traceparent,
        })
    }
}

/// Cortex `/v1/mesh/message-status` state (`WS-X1` / `WS-X2` `PoolMessageStatusView`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PoolMessageStatus {
    /// Queued in the Redis stream, not yet claimed (`status == "unread"`).
    Unread,
    /// Claimed and in-flight on a pool member (`status == "pending"`).
    Pending,
    /// Stream entry acknowledged by the consumer group (`status == "acked"`).
    Acked,
    /// Exceeded `max_deliveries` or dead-lettered by Cortex (`status == "dead_lettered"`).
    DeadLettered,
}

impl PoolMessageStatus {
    /// Parse a wire status string from `GET /v1/mesh/message-status`.
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim() {
            "unread" => Some(Self::Unread),
            "pending" => Some(Self::Pending),
            "acked" => Some(Self::Acked),
            "dead_lettered" => Some(Self::DeadLettered),
            _ => None,
        }
    }

    /// Return the canonical Cortex wire string.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unread => "unread",
            Self::Pending => "pending",
            Self::Acked => "acked",
            Self::DeadLettered => "dead_lettered",
        }
    }
}

/// Pure deterministic resolution decision for an in-flight Cortex Mesh Pool task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobResolution {
    /// Job remains queued or actively claimed in the pool.
    InFlight,
    /// Worker reported terminal completion (`outcome == "ok"`).
    Completed,
    /// Transient failure or dead-letter eligible for deterministic retry.
    Requeue {
        /// Incremented attempt counter (`attempt + 1`).
        next_attempt: u32,
    },
    /// Terminal failure (`outcome == "permanent"` or retry budget exhausted).
    DeadLetter {
        /// Terminal failure reason.
        reason: String,
    },
}

/// Pure state reconciliation function combining a Cortex `/v1/mesh/message-status`
/// status and an optional inbound `mesh-pool` result `MSG` (`SPEC-0003`).
#[must_use]
pub fn resolve_pool_job_state(
    status: PoolMessageStatus,
    result: Option<&PoolResultRefs>,
    attempt: u32,
    max_retries: u32,
) -> JobResolution {
    if let Some(res) = result {
        match res.outcome {
            PoolResultOutcome::Ok => return JobResolution::Completed,
            PoolResultOutcome::Transient => {
                if attempt < max_retries {
                    return JobResolution::Requeue {
                        next_attempt: attempt.saturating_add(1),
                    };
                }
                let reason = res
                    .error
                    .clone()
                    .unwrap_or_else(|| String::from("transient pool failure exhausted retries"));
                return JobResolution::DeadLetter { reason };
            }
            PoolResultOutcome::Permanent => {
                let reason = res
                    .error
                    .clone()
                    .unwrap_or_else(|| String::from("permanent pool worker failure"));
                return JobResolution::DeadLetter { reason };
            }
        }
    }

    match status {
        PoolMessageStatus::Unread | PoolMessageStatus::Pending | PoolMessageStatus::Acked => {
            JobResolution::InFlight
        }
        PoolMessageStatus::DeadLettered => {
            if attempt < max_retries {
                JobResolution::Requeue {
                    next_attempt: attempt.saturating_add(1),
                }
            } else {
                JobResolution::DeadLetter {
                    reason: format!("cortex pool dead-lettered after {attempt} attempt(s)"),
                }
            }
        }
    }
}

/// Compute the maximum number of concurrent in-flight pool jobs permitted for a capability pool.
#[must_use]
pub const fn max_pool_inflight(
    pool_consumers: usize,
    lane_capacity: usize,
    per_consumer_inflight: usize,
) -> usize {
    let consumers = if pool_consumers == 0 {
        1
    } else {
        pool_consumers
    };
    let per_consumer = if per_consumer_inflight == 0 {
        1
    } else {
        per_consumer_inflight
    };
    let scaled = consumers.saturating_mul(per_consumer);
    if scaled < lane_capacity {
        scaled
    } else {
        lane_capacity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_state() {
        let state = EpState::default();
        assert_eq!(state.version, 1);
        assert_eq!(state.id, "initial");
    }

    #[test]
    fn closed_and_draining_valves_hold_impulses() {
        let quota = TickQuota::full(100);
        assert_eq!(
            evaluate_impulse(ValveState::Closed, &quota, 10),
            ReleaseDecision::Hold {
                reason: HoldReason::ValveClosed
            }
        );
        assert_eq!(
            evaluate_impulse(ValveState::Draining, &quota, 10),
            ReleaseDecision::Hold {
                reason: HoldReason::ValveDraining
            }
        );
    }

    #[test]
    fn open_valve_permits_within_quota_and_computes_remaining() {
        let quota = TickQuota::new(100, 60, 25);
        let decision = evaluate_impulse(ValveState::Open, &quota, 20);
        assert!(decision.is_permitted());
        assert_eq!(decision, ReleaseDecision::Permit { remaining: 40 });
    }

    #[test]
    fn open_valve_holds_zero_or_excessive_impulse() {
        let quota = TickQuota::new(100, 60, 25);

        assert_eq!(
            evaluate_impulse(ValveState::Open, &quota, 0),
            ReleaseDecision::Hold {
                reason: HoldReason::ZeroImpulseRequested
            }
        );

        assert_eq!(
            evaluate_impulse(ValveState::Open, &quota, 30),
            ReleaseDecision::Hold {
                reason: HoldReason::ExceedsMaxPerImpulse {
                    requested: 30,
                    max_per_impulse: 25,
                }
            }
        );
    }

    #[test]
    fn open_valve_holds_when_quota_exhausted_and_recovers_on_refill() {
        let mut quota = TickQuota::new(50, 10, 25);
        assert_eq!(
            evaluate_impulse(ValveState::Open, &quota, 15),
            ReleaseDecision::Hold {
                reason: HoldReason::QuotaExhausted {
                    requested: 15,
                    available: 10,
                }
            }
        );

        quota.refill(20);
        assert_eq!(quota.available, 30);
        assert_eq!(
            evaluate_impulse(ValveState::Open, &quota, 15),
            ReleaseDecision::Permit { remaining: 15 }
        );

        // Refill saturates at capacity.
        quota.refill(100);
        assert_eq!(quota.available, 50);
    }

    #[test]
    fn tick_and_cadence_arithmetic_is_deterministic() {
        assert_eq!(Cadence::new(0), Err(EpError::ZeroCadenceInterval));
        let cadence = Cadence::new(5).unwrap();
        assert!(cadence.is_due(Tick(0)));
        assert!(!cadence.is_due(Tick(3)));
        assert!(cadence.is_due(Tick(10)));
        assert_eq!(cadence.next_tick(Tick(0)), Tick(5));
        assert_eq!(cadence.next_tick(Tick(3)), Tick(5));
        assert_eq!(cadence.next_tick(Tick(5)), Tick(10));
    }

    #[test]
    fn token_bucket_refills_and_meters_on_integer_ticks() {
        assert_eq!(
            TokenBucket::new(0, 5, Tick::ZERO),
            Err(EpError::ZeroCapacity)
        );
        let mut bucket = TokenBucket::new(10, 2, Tick(0)).unwrap();

        assert_eq!(
            bucket.try_acquire(Tick(0), 6),
            ValveDecision::Permit { remaining: 4 }
        );
        assert_eq!(
            bucket.try_acquire(Tick(0), 5),
            ValveDecision::Hold {
                reason: HoldReason::QuotaExhausted {
                    requested: 5,
                    available: 4,
                }
            }
        );

        // Advance 2 ticks -> +4 tokens (4 + 4 = 8 available).
        assert_eq!(
            bucket.try_acquire(Tick(2), 5),
            ValveDecision::Permit { remaining: 3 }
        );

        // Non-monotonic tick is a no-op refill.
        bucket.refill_to(Tick(1));
        assert_eq!(bucket.available, 3);

        // Advance to Tick(10) -> saturates at capacity 10.
        bucket.refill_to(Tick(10));
        assert_eq!(bucket.available, 10);
    }

    #[test]
    fn impulse_stamp_orders_by_tick_then_sequence() {
        let s1 = ImpulseStamp::new(Tick(1), ImpulseSeq(0));
        let s2 = ImpulseStamp::new(Tick(1), ImpulseSeq(1));
        let s3 = ImpulseStamp::new(Tick(2), ImpulseSeq(0));
        assert!(s1 < s2);
        assert!(s2 < s3);
    }

    #[test]
    fn detect_stagnation_classifies_all_four_patterns() {
        assert_eq!(
            detect_stagnation(&[42, 42, 42], &[10, 10, 10]),
            Some((StagnationPattern::Spinning, ControlDirective::Retry))
        );
        assert_eq!(
            detect_stagnation(&[1, 2, 1, 2], &[5, 5, 5]),
            Some((StagnationPattern::Oscillation, ControlDirective::Cancel))
        );
        assert_eq!(
            detect_stagnation(&[1, 2, 3], &[0, 0, 0]),
            Some((StagnationPattern::NoDrift, ControlDirective::Wait))
        );
        assert_eq!(
            detect_stagnation(&[1, 2, 3], &[10, 4, 1]),
            Some((
                StagnationPattern::DiminishingReturns,
                ControlDirective::Converge
            ))
        );
        assert_eq!(detect_stagnation(&[1, 2, 3], &[5, 8, 12]), None);
    }

    #[test]
    fn select_lateral_persona_rotates_deterministically_by_pattern_and_attempt() {
        assert_eq!(
            select_lateral_persona(StagnationPattern::Spinning, 0),
            LateralPersona::Contrarian
        );
        assert_eq!(
            select_lateral_persona(StagnationPattern::Spinning, 1),
            LateralPersona::Hacker
        );
        assert_eq!(
            select_lateral_persona(StagnationPattern::Oscillation, 0),
            LateralPersona::Architect
        );
        assert_eq!(
            select_lateral_persona(StagnationPattern::Oscillation, 1),
            LateralPersona::Contrarian
        );
        assert_eq!(
            select_lateral_persona(StagnationPattern::NoDrift, 0),
            LateralPersona::Hacker
        );
        assert_eq!(
            select_lateral_persona(StagnationPattern::NoDrift, 2),
            LateralPersona::Researcher
        );
        assert_eq!(
            select_lateral_persona(StagnationPattern::DiminishingReturns, 0),
            LateralPersona::Simplifier
        );
    }

    #[test]
    fn detect_stagnation_with_persona_emits_inject_persona_directive() {
        assert_eq!(
            detect_stagnation_with_persona(&[42, 42, 42], &[10, 10, 10], 0),
            Some((
                StagnationPattern::Spinning,
                ControlDirective::InjectPersona {
                    persona: LateralPersona::Contrarian,
                    reason: StagnationPattern::Spinning,
                }
            ))
        );
        assert_eq!(
            detect_stagnation_with_persona(&[1, 2, 1, 2], &[5, 5, 5], 1),
            Some((
                StagnationPattern::Oscillation,
                ControlDirective::InjectPersona {
                    persona: LateralPersona::Contrarian,
                    reason: StagnationPattern::Oscillation,
                }
            ))
        );
        assert_eq!(
            detect_stagnation_with_persona(&[1, 2, 3], &[5, 8, 12], 0),
            None
        );
    }

    #[test]
    fn vertex_quota_preset_bucket_and_429_backoff_are_deterministic() {
        assert!(is_vertex_provider("vertex"));
        assert!(is_vertex_provider("google-vertex"));
        assert!(is_vertex_provider("vertex-ai"));
        assert!(!is_vertex_provider("gateway"));

        let preset = VertexQuotaPreset::ceres_default();
        let mut bucket = TokenBucket::vertex_default(Tick(0));
        assert_eq!(bucket.capacity, preset.burst_capacity);
        assert_eq!(bucket.available, 15);
        assert!(bucket.try_consume(15, 0).is_ok());
        assert_eq!(
            bucket.try_consume(1, 0),
            Err(RateLimited {
                deficit_tokens: 1,
                retry_after_ticks: 1,
                retry_after_ms: 1,
            })
        );

        assert_eq!(preset.backoff_ticks_for_429(0), 0);
        assert_eq!(preset.backoff_ticks_for_429(1), 20);
        assert_eq!(preset.backoff_ticks_for_429(2), 40);
        assert_eq!(preset.backoff_ticks_for_429(3), 80);
        assert_eq!(preset.backoff_ticks_for_429(10), 300);
    }

    #[test]
    fn capability_slug_validates_and_formats_targets() {
        let slug = CapabilitySlug::new("code_review-v1").unwrap();
        assert_eq!(slug.as_str(), "code_review-v1");
        assert_eq!(slug.workstream_target(), "workstreams/code_review-v1");
        assert_eq!(slug.pool_firewall_entry(), "pool:code_review-v1");
        assert_eq!(slug.pool_address(), "pool:code_review-v1");

        assert!(CapabilitySlug::new("").is_err());
        assert!(CapabilitySlug::new("Upper_Case").is_err());
        assert!(CapabilitySlug::new("bad/slash").is_err());
        assert!(CapabilitySlug::new("a".repeat(65)).is_err());
    }

    #[test]
    fn pool_delivery_refs_enforces_stream_seq_and_struct_bounds() {
        let pool = CapabilitySlug::new("verify").unwrap();
        assert!(PoolDeliveryRefs::new(pool.clone(), 1, "", 1, None).is_err());
        assert!(PoolDeliveryRefs::new(pool.clone(), 0, "task-1", 1, None).is_err());
        assert!(PoolDeliveryRefs::new(pool.clone(), 1, "task-1", 0, None).is_err());
        assert!(
            PoolDeliveryRefs::new(pool.clone(), MAX_SAFE_STRUCT_INT + 1, "task-1", 1, None)
                .is_err()
        );
        let valid = PoolDeliveryRefs::new(pool, MAX_SAFE_STRUCT_INT, "task-1", 1, None).unwrap();
        assert_eq!(valid.stream_seq, MAX_SAFE_STRUCT_INT);
    }

    #[test]
    fn resolve_pool_job_state_handles_outcomes_and_dead_letters() {
        let ok_res = PoolResultRefs::new(
            77,
            "task-1",
            1,
            PoolResultOutcome::Ok,
            None,
            Some(String::from("00-abc-def-01")),
        )
        .unwrap();
        assert_eq!(
            resolve_pool_job_state(PoolMessageStatus::Acked, Some(&ok_res), 1, 3),
            JobResolution::Completed
        );

        let transient_res = PoolResultRefs::new(
            77,
            "task-1",
            1,
            PoolResultOutcome::Transient,
            Some(String::from("gateway timeout")),
            None,
        )
        .unwrap();
        assert_eq!(
            resolve_pool_job_state(PoolMessageStatus::Acked, Some(&transient_res), 1, 3),
            JobResolution::Requeue { next_attempt: 2 }
        );

        assert_eq!(
            resolve_pool_job_state(PoolMessageStatus::DeadLettered, None, 1, 3),
            JobResolution::Requeue { next_attempt: 2 }
        );
        assert_eq!(
            resolve_pool_job_state(PoolMessageStatus::DeadLettered, None, 3, 3),
            JobResolution::DeadLetter {
                reason: String::from("cortex pool dead-lettered after 3 attempt(s)"),
            }
        );

        assert_eq!(
            resolve_pool_job_state(PoolMessageStatus::Pending, None, 1, 3),
            JobResolution::InFlight
        );
        assert_eq!(max_pool_inflight(4, 100, 2), 8);
        assert_eq!(max_pool_inflight(4, 6, 2), 6);
    }

    fn next_u64(state: &mut u64) -> u64 {
        let mut x = *state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        *state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    #[test]
    fn prop_token_bucket_never_exceeds_capacity_or_underflows() {
        let mut seed = 0xDEAD_BEEF_CAFE_BABEu64;
        for _ in 0..2000 {
            let cap = (next_u64(&mut seed) % 500) + 1;
            let refill = next_u64(&mut seed) % 50;
            let init = next_u64(&mut seed) % (cap + 50);
            let mut bucket = TokenBucket::with_initial(cap, init, refill, Tick(0));
            assert!(bucket.available <= cap);

            let mut now = 0u64;
            for _ in 0..20 {
                // Sometimes advance clock, sometimes keep same or earlier tick
                let delta = (next_u64(&mut seed) % 10) as i64 - 2;
                let probe_tick = if delta < 0 {
                    now.saturating_sub(delta.unsigned_abs())
                } else {
                    now.saturating_add(delta as u64)
                };
                if probe_tick > now {
                    now = probe_tick;
                }
                let cost = next_u64(&mut seed) % (cap + 20);
                let before = bucket.tokens(probe_tick);
                assert!(before <= cap);
                match bucket.try_acquire(Tick(probe_tick), cost) {
                    ValveDecision::Permit { remaining } => {
                        assert!(cost > 0);
                        assert!(cost <= before);
                        assert_eq!(remaining, before - cost);
                        assert_eq!(bucket.available, remaining);
                    }
                    ValveDecision::Hold { .. } => {
                        assert!(cost == 0 || cost > before);
                        assert_eq!(bucket.available, before);
                    }
                }
            }
        }
    }

    #[test]
    fn prop_resolve_pool_job_state_totality_and_retry_bounds() {
        let statuses = [
            PoolMessageStatus::Unread,
            PoolMessageStatus::Pending,
            PoolMessageStatus::Acked,
            PoolMessageStatus::DeadLettered,
        ];
        let outcomes = [
            None,
            Some(PoolResultOutcome::Ok),
            Some(PoolResultOutcome::Transient),
            Some(PoolResultOutcome::Permanent),
        ];

        for status in statuses {
            for outcome in outcomes {
                for attempt in 1..=6u32 {
                    for max_retries in 1..=5u32 {
                        let res_refs = outcome.map(|o| {
                            PoolResultRefs::new(1, "task-prop", attempt, o, None, None).unwrap()
                        });
                        let resolution =
                            resolve_pool_job_state(status, res_refs.as_ref(), attempt, max_retries);
                        match (outcome, status, resolution) {
                            (Some(PoolResultOutcome::Ok), _, JobResolution::Completed) => {}
                            (
                                Some(PoolResultOutcome::Permanent),
                                _,
                                JobResolution::DeadLetter { .. },
                            ) => {}
                            (
                                Some(PoolResultOutcome::Transient),
                                _,
                                JobResolution::Requeue { next_attempt },
                            ) => {
                                assert!(attempt < max_retries);
                                assert_eq!(next_attempt, attempt + 1);
                            }
                            (
                                Some(PoolResultOutcome::Transient),
                                _,
                                JobResolution::DeadLetter { .. },
                            ) => {
                                assert!(attempt >= max_retries);
                            }
                            (
                                None,
                                PoolMessageStatus::Unread
                                | PoolMessageStatus::Pending
                                | PoolMessageStatus::Acked,
                                JobResolution::InFlight,
                            ) => {}
                            (
                                None,
                                PoolMessageStatus::DeadLettered,
                                JobResolution::Requeue { next_attempt },
                            ) => {
                                assert!(attempt < max_retries);
                                assert_eq!(next_attempt, attempt + 1);
                            }
                            (
                                None,
                                PoolMessageStatus::DeadLettered,
                                JobResolution::DeadLetter { .. },
                            ) => {
                                assert!(attempt >= max_retries);
                            }
                            other => panic!("unexpected state resolution: {other:?}"),
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn prop_max_pool_inflight_bounded_by_provider_cap() {
        let mut seed = 0x1234_5678_9ABC_DEF0u64;
        for _ in 0..2000 {
            let members = (next_u64(&mut seed) % 64) as usize;
            let cap = (next_u64(&mut seed) % 128) as usize;
            let mult = (next_u64(&mut seed) % 8) as usize;
            let bound = max_pool_inflight(members, cap, mult);
            assert!(bound <= cap);
            if cap == 0 {
                assert_eq!(bound, 0);
            } else {
                assert!(bound >= 1);
                assert_eq!(bound, members.max(1).saturating_mul(mult.max(1)).min(cap));
            }
        }
    }
}
