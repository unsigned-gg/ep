//! Waker integration: TCP wake proxy client for scale-to-zero microVMs.
//!
//! Escapement dispatches jobs to waker-managed targets — workloads that
//! scale to zero and need to be woken via waker's TCP accept-and-hold
//! proxy before dispatch can proceed.
//!
//! This module provides a zero-dependency TCP client that connects to
//! waker's listener, splices bytes, and reports readiness. The actual
//! waker deployment (Helm chart, Service) is in `unsigned-gg/waker`.

use std::fmt;
use std::io;
use std::net::TcpStream;
use std::time::Duration;

/// Configuration for a waker target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WakerTarget {
    /// The waker listener address (e.g. `waker.unsigned.svc:10023`).
    pub listen_addr: String,
    /// The target workload name (e.g. `aria-vm`).
    pub workload: String,
    /// The namespace of the target workload.
    pub namespace: String,
}

impl WakerTarget {
    /// Create a waker target from its listen address, workload, and namespace.
    #[must_use]
    pub fn new(
        listen_addr: impl Into<String>,
        workload: impl Into<String>,
        namespace: impl Into<String>,
    ) -> Self {
        Self {
            listen_addr: listen_addr.into(),
            workload: workload.into(),
            namespace: namespace.into(),
        }
    }
}

/// Result of a wake attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WakeResult {
    /// The workload was already running and the connection succeeded.
    AlreadyRunning,
    /// The workload was scaled from zero and is now ready.
    Waked,
    /// The wake timed out after `timeout_ms` milliseconds.
    Timeout {
        /// Milliseconds waited before the wake timed out.
        timeout_ms: u64,
    },
    /// A connection error occurred.
    Failed(String),
}

impl fmt::Display for WakeResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyRunning => write!(f, "already running"),
            Self::Waked => write!(f, "waked from zero"),
            Self::Timeout { timeout_ms } => write!(f, "timeout after {timeout_ms}ms"),
            Self::Failed(msg) => write!(f, "failed: {msg}"),
        }
    }
}

/// Wake a waker-managed target by connecting to its TCP listener.
///
/// Waker holds the connection until the backend is ready, then splices
/// bytes. A successful `connect` means the backend answered — the workload
/// is up.
///
/// # Arguments
/// * `target` — The waker target to wake.
/// * `timeout_ms` — Maximum time to wait for the connection (capped at 120s).
///
/// # Errors
/// Returns [`WakeResult::Failed`] on connection errors.
#[must_use]
pub fn wake_and_wait(target: &WakerTarget, timeout_ms: u64) -> WakeResult {
    use std::net::ToSocketAddrs;
    let timeout = Duration::from_millis(timeout_ms.min(120_000));

    // Resolve the listen address to a SocketAddr.
    let addr = match target.listen_addr.to_socket_addrs() {
        Ok(mut addrs) => match addrs.next() {
            Some(addr) => addr,
            None => {
                return WakeResult::Failed(format!("no addresses for {}", target.listen_addr));
            }
        },
        Err(e) => return WakeResult::Failed(e.to_string()),
    };

    match TcpStream::connect_timeout(&addr, timeout) {
        Ok(_) => WakeResult::Waked,
        Err(e) if e.kind() == io::ErrorKind::TimedOut => WakeResult::Timeout { timeout_ms },
        Err(e) => WakeResult::Failed(e.to_string()),
    }
}

/// Pluggable probe interface for waking scale-to-zero targets.
pub trait WakerProbe {
    /// Attempt to wake `target` within `timeout_ms`.
    fn wake(&self, target: &WakerTarget, timeout_ms: u64) -> WakeResult;
}

/// Production TCP probe that delegates to [`wake_and_wait`].
#[derive(Debug, Default, Clone, Copy)]
pub struct TcpWakerProbe;

impl WakerProbe for TcpWakerProbe {
    fn wake(&self, target: &WakerTarget, timeout_ms: u64) -> WakeResult {
        wake_and_wait(target, timeout_ms)
    }
}

/// Error returned when a waker health gate blocks task dispatch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WakerGateError {
    /// Target has exceeded the consecutive wake failure threshold (`CrashLoopBackOff`).
    CrashLoopBackOff {
        /// Capability name gated by the waker.
        capability: String,
        /// Number of consecutive failed wake attempts.
        consecutive_failures: u32,
    },
    /// A wake attempt timed out or failed before reaching readiness.
    WakeFailed {
        /// Capability name gated by the waker.
        capability: String,
        /// Underlying wake failure or timeout result.
        result: WakeResult,
        /// Updated consecutive failure count.
        consecutive_failures: u32,
    },
}

impl fmt::Display for WakerGateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CrashLoopBackOff {
                capability,
                consecutive_failures,
            } => write!(
                f,
                "waker CrashLoopBackOff for capability '{capability}' ({consecutive_failures} consecutive failures)"
            ),
            Self::WakeFailed {
                capability,
                result,
                consecutive_failures,
            } => write!(
                f,
                "waker failed for capability '{capability}' (attempt {consecutive_failures}): {result}"
            ),
        }
    }
}

impl std::error::Error for WakerGateError {}

/// Tracks scale-to-zero [`WakerTarget`] bindings and `CrashLoopBackOff` circuit breaking per capability.
#[derive(Debug, Clone)]
pub struct WakerHealthGate {
    targets: std::collections::BTreeMap<String, WakerTarget>,
    consecutive_failures: std::collections::BTreeMap<String, u32>,
    max_consecutive_failures: u32,
}

impl Default for WakerHealthGate {
    fn default() -> Self {
        Self::new(3)
    }
}

impl WakerHealthGate {
    /// Create a new waker health gate with the given `CrashLoopBackOff` failure threshold.
    #[must_use]
    pub fn new(max_consecutive_failures: u32) -> Self {
        Self {
            targets: std::collections::BTreeMap::new(),
            consecutive_failures: std::collections::BTreeMap::new(),
            max_consecutive_failures: max_consecutive_failures.max(1),
        }
    }

    /// Bind a capability to a scale-to-zero [`WakerTarget`].
    pub fn register_target(&mut self, capability: impl Into<String>, target: WakerTarget) {
        let cap = capability.into();
        self.consecutive_failures.insert(cap.clone(), 0);
        self.targets.insert(cap, target);
    }

    /// Return the registered [`WakerTarget`] for `capability`, if any.
    #[must_use]
    pub fn target(&self, capability: &str) -> Option<&WakerTarget> {
        self.targets.get(capability)
    }

    /// Return the current consecutive failure count for `capability`.
    #[must_use]
    pub fn consecutive_failures(&self, capability: &str) -> u32 {
        self.consecutive_failures
            .get(capability)
            .copied()
            .unwrap_or(0)
    }

    /// Return `true` if `capability` is currently tripped in `CrashLoopBackOff`.
    #[must_use]
    pub fn is_crash_looping(&self, capability: &str) -> bool {
        self.consecutive_failures(capability) >= self.max_consecutive_failures
    }

    /// Reset the failure counter for `capability`.
    pub fn reset_failures(&mut self, capability: &str) {
        if let Some(count) = self.consecutive_failures.get_mut(capability) {
            *count = 0;
        }
    }

    /// Ensure the target for `capability` is awake before dispatching work.
    ///
    /// If `capability` has no registered [`WakerTarget`], returns `Ok(WakeResult::AlreadyRunning)`.
    ///
    /// # Errors
    /// Returns [`WakerGateError::CrashLoopBackOff`] if the target has already tripped the failure
    /// threshold, or [`WakerGateError::WakeFailed`] if the probe fails or times out.
    pub fn ensure_awake(
        &mut self,
        capability: &str,
        probe: &dyn WakerProbe,
        timeout_ms: u64,
    ) -> Result<WakeResult, WakerGateError> {
        let Some(target) = self.targets.get(capability).cloned() else {
            return Ok(WakeResult::AlreadyRunning);
        };

        let current_failures = self.consecutive_failures(capability);
        if current_failures >= self.max_consecutive_failures {
            return Err(WakerGateError::CrashLoopBackOff {
                capability: capability.to_string(),
                consecutive_failures: current_failures,
            });
        }

        let outcome = probe.wake(&target, timeout_ms);
        match outcome {
            WakeResult::AlreadyRunning | WakeResult::Waked => {
                self.consecutive_failures.insert(capability.to_string(), 0);
                Ok(outcome)
            }
            WakeResult::Timeout { .. } | WakeResult::Failed(_) => {
                let next = current_failures.saturating_add(1);
                self.consecutive_failures
                    .insert(capability.to_string(), next);
                Err(WakerGateError::WakeFailed {
                    capability: capability.to_string(),
                    result: outcome,
                    consecutive_failures: next,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StaticProbe(WakeResult);

    impl WakerProbe for StaticProbe {
        fn wake(&self, _target: &WakerTarget, _timeout_ms: u64) -> WakeResult {
            self.0.clone()
        }
    }

    #[test]
    fn waker_target_construction() {
        let target = WakerTarget::new("waker.svc:10023", "aria-vm", "aria-vm");
        assert_eq!(target.listen_addr, "waker.svc:10023");
        assert_eq!(target.workload, "aria-vm");
        assert_eq!(target.namespace, "aria-vm");
    }

    #[test]
    fn wake_result_display() {
        assert_eq!(WakeResult::AlreadyRunning.to_string(), "already running");
        assert_eq!(WakeResult::Waked.to_string(), "waked from zero");
        assert!(WakeResult::Timeout {
            timeout_ms: 120_000
        }
        .to_string()
        .contains("120000"));
        assert!(WakeResult::Failed("refused".into())
            .to_string()
            .contains("refused"));
    }

    #[test]
    fn wake_unresolvable_fails() {
        let target = WakerTarget::new("nonexistent.invalid:9999", "test", "default");
        let result = wake_and_wait(&target, 1000);
        assert!(matches!(
            result,
            WakeResult::Failed(_) | WakeResult::Timeout { .. }
        ));
    }

    #[test]
    fn wake_timeout_capped() {
        // The function should cap at 120s even if a longer timeout is given.
        // Just verify it doesn't panic.
        let target = WakerTarget::new("nonexistent.invalid:9999", "test", "default");
        let _result = wake_and_wait(&target, 999_999_999);
    }

    #[test]
    fn health_gate_trips_crash_loop_backoff_after_threshold_and_recovers_on_reset() {
        let mut gate = WakerHealthGate::new(2);
        gate.register_target(
            "gpu-infer",
            WakerTarget::new("waker.svc:10023", "infer-vm", "default"),
        );

        let fail_probe = StaticProbe(WakeResult::Failed("connection refused".into()));
        assert!(matches!(
            gate.ensure_awake("gpu-infer", &fail_probe, 500),
            Err(WakerGateError::WakeFailed {
                consecutive_failures: 1,
                ..
            })
        ));
        assert!(!gate.is_crash_looping("gpu-infer"));

        assert!(matches!(
            gate.ensure_awake("gpu-infer", &fail_probe, 500),
            Err(WakerGateError::WakeFailed {
                consecutive_failures: 2,
                ..
            })
        ));
        assert!(gate.is_crash_looping("gpu-infer"));

        // Subsequent attempts short-circuit with CrashLoopBackOff even if probe would succeed.
        let ok_probe = StaticProbe(WakeResult::Waked);
        assert_eq!(
            gate.ensure_awake("gpu-infer", &ok_probe, 500),
            Err(WakerGateError::CrashLoopBackOff {
                capability: "gpu-infer".into(),
                consecutive_failures: 2,
            })
        );

        // Operator/reconciler reset clears CrashLoopBackOff and allows wake.
        gate.reset_failures("gpu-infer");
        assert_eq!(
            gate.ensure_awake("gpu-infer", &ok_probe, 500),
            Ok(WakeResult::Waked)
        );
        assert_eq!(gate.consecutive_failures("gpu-infer"), 0);
    }
}
