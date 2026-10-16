//! escapement-core provides primitives for autonomous agent orchestration.
//!
//! # Architecture
//! The engine includes an agent [`registry`], a priority [`dispatch`] queue, and a DAG [`plan`] solver.
//! It tracks [`custody`] chains and enforces typed [`protocol`] messages.
//! The system controls throughput with [`metering`], priority [`lanes`], [`backpressure`], and [`budget`] limits.
//! It supports state [`persistence`], failure [`resilience`], [`tracing`], and [`telemetry`].
//! Callers supply time in milliseconds to keep execution deterministic.

pub mod backpressure;
pub mod budget;
pub mod custody;
pub mod dispatch;
pub mod lanes;
pub mod merge;
pub mod metering;
pub mod persistence;
pub mod plan;
pub mod presence;
pub mod protocol;
pub mod registry;
pub mod resilience;
pub mod telemetry;
pub mod tracing;
pub mod waker;

pub use backpressure::{
    BackpressureDecision, ProviderCapacity, ProviderHealth, ProviderRegistry, UnknownProvider,
};
pub use budget::{BudgetDecision, BudgetTracker, UnknownBudgetProvider};
pub use custody::{CustodyChain, CustodyError, CustodyRef, DispatchRecord};
pub use dispatch::{DispatchError, Dispatcher, Task, TaskId, TaskState};
pub use ep_domain::*;
pub use lanes::{LaneError, LaneScheduler, PriorityLane};
pub use merge::{has_overlap, merge_all, merge_overlapping, overlapping_paths, Changeset};
pub use metering::{ConcurrencyExceeded, ConcurrencyGuard};
pub use persistence::{
    deserialize_entry, reconstruct, replay, serialize_entry, write_entry, PersistenceError,
    RecoveredState, WalEntry, WalOp,
};
pub use plan::{Plan, PlanError, PlanNode, PlanNodeId};
pub use presence::{Presence, PresenceBridge, RosterEntry};
pub use protocol::{
    AdmissionDecision, DispatchReceipt, PlanNodeSpec, PlanSubmission, ProtocolError,
    ProtocolMessage, SettleReceipt, PROTOCOL_VERSION,
};
pub use registry::{AgentId, AgentRecord, AgentState, Registry, RegistryError};
pub use resilience::{RetryPolicy, RetryReason};
pub use telemetry::{
    DispatchEvent, DispatchMetrics, OtlpConfig, TelemetryLog, TelemetryMetric, TelemetrySpan,
};
pub use waker::{wake_and_wait, WakeResult, WakerTarget};

pub use tracing::{OtlpExporter, Span, SpanBatch, TraceContext};
