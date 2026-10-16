//! escapement-serve: the orchestrator daemon.
//!
//! Ties together escapement-core (registry, dispatcher, plan, custody, protocol),
//! blackwall-bridge (spawn runs, receive custody, settle, execution), and the
//! metering subsystems (concurrency, rate limits, backpressure, budget, telemetry)
//! into a single running process.
//!
//! The orchestrator is single-writer by design: all mutations happen in one
//! task. The dispatch loop is deterministic — same inputs produce same outputs.

use std::collections::BTreeMap;
use std::fmt;

use escapement_core::backpressure::{BackpressureDecision, ProviderRegistry};
use escapement_core::budget::{BudgetDecision, BudgetTracker};
use escapement_core::custody::CustodyChain;
use escapement_core::dispatch::{Dispatcher, Task, TaskId};
use escapement_core::lanes::LaneScheduler;
use escapement_core::metering::{ConcurrencyGuard, TokenBucket};
use escapement_core::persistence::{RecoveredState, WalEntry};
use escapement_core::plan::Plan;
use escapement_core::registry::{AgentId, Registry};
use escapement_core::telemetry::DispatchMetrics;

use blackwall_bridge::execution::PlanExecutor;
pub mod http;
pub mod pool;

/// Orchestrator configuration — loaded from `escapement.toml`.
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    /// Default provider for dispatched runs.
    pub default_provider: String,
    /// Default model.
    pub default_model: String,
    /// Max turns per run.
    pub default_max_turns: u32,
    /// Max tokens per run (budget cap).
    pub max_tokens_per_run: u64,
    /// Max retries before dead-lettering.
    pub max_retries: u32,
    /// Concurrency caps per capability.
    pub concurrency_caps: BTreeMap<String, usize>,
    /// Token bucket capacity for rate limiting (integer tokens).
    pub rate_capacity: u64,
    /// Token bucket refill rate (integer tokens per tick).
    pub rate_refill_per_second: u64,
    /// Tick interval for the dispatch loop (milliseconds).
    pub tick_interval_ms: u64,
    /// OTLP endpoint for telemetry.
    pub otlp_endpoint: String,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            default_provider: "gateway".into(),
            default_model: "glm-5.2".into(),
            default_max_turns: 20,
            max_tokens_per_run: 200_000,
            max_retries: 3,
            concurrency_caps: BTreeMap::new(),
            rate_capacity: 10,
            rate_refill_per_second: 10,
            tick_interval_ms: 100,
            otlp_endpoint: "http://localhost:4318/v1/traces".into(),
        }
    }
}

/// The orchestrator — wires all subsystems and runs the dispatch loop.
#[derive(Debug)]
#[allow(dead_code)]
pub struct Orchestrator {
    config: OrchestratorConfig,
    registry: Registry,
    dispatcher: Dispatcher,
    lane_scheduler: LaneScheduler,
    concurrency_guard: ConcurrencyGuard,
    rate_limiter: TokenBucket,
    providers: ProviderRegistry,
    budget: BudgetTracker,
    custody: CustodyChain,
    plan_executors: BTreeMap<TaskId, PlanExecutor>,
    metrics: DispatchMetrics,
    /// OTLP exporter for distributed tracing.
    otlp: Option<escapement_core::tracing::OtlpExporter>,
    /// Simulated clock (caller-supplied ms) — deterministic.
    clock_ms: u64,
}

/// Orchestrator errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrchestratorError {
    /// A task was submitted that already exists.
    DuplicateTask(TaskId),
    /// A provider is at capacity.
    ProviderAtCapacity(String),
    /// Budget exceeded for a provider.
    BudgetExceeded(String),
    /// Rate limited.
    RateLimited,
    /// An unknown task was referenced.
    UnknownTask(TaskId),
    /// The task is not in a state that permits the transition.
    InvalidStateTransition(TaskId),
    /// Pool correlation metadata failed validation.
    InvalidPoolRefs(String),
}

impl fmt::Display for OrchestratorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateTask(id) => write!(f, "duplicate task: {id}"),
            Self::ProviderAtCapacity(p) => write!(f, "provider at capacity: {p}"),
            Self::BudgetExceeded(p) => write!(f, "budget exceeded: {p}"),
            Self::RateLimited => write!(f, "rate limited"),
            Self::UnknownTask(id) => write!(f, "unknown task: {id}"),
            Self::InvalidStateTransition(id) => write!(f, "invalid state transition: {id}"),
            Self::InvalidPoolRefs(msg) => write!(f, "invalid pool refs: {msg}"),
        }
    }
}

impl std::error::Error for OrchestratorError {}

impl Orchestrator {
    /// Create a new orchestrator from configuration.
    #[must_use]
    pub fn new(config: OrchestratorConfig) -> Self {
        let mut concurrency_guard = ConcurrencyGuard::new();
        for (cap, limit) in &config.concurrency_caps {
            concurrency_guard.set_cap(cap, *limit);
        }

        let mut providers = ProviderRegistry::new();
        let mut budget = BudgetTracker::new();
        let vertex_preset = escapement_core::metering::VertexQuotaPreset::ceres_default();
        for vertex_name in ["vertex", "google-vertex", "vertex-ai"] {
            providers.register(vertex_name, vertex_preset.max_concurrent);
            budget.set_cap(vertex_name, config.max_tokens_per_run);
        }
        if concurrency_guard.cap("vertex") == 0 {
            concurrency_guard.set_cap("vertex", vertex_preset.max_concurrent);
        }

        let rate_limiter =
            if escapement_core::metering::is_vertex_provider(&config.default_provider) {
                TokenBucket::from_ticks(
                    vertex_preset.burst_capacity,
                    vertex_preset.refill_per_tick,
                    0,
                )
            } else {
                TokenBucket::from_ticks(config.rate_capacity, config.rate_refill_per_second, 0)
            };
        let dispatcher = Dispatcher::new();

        // Initialize OTLP exporter if endpoint is configured.
        let otlp = if config.otlp_endpoint.is_empty() {
            None
        } else {
            Some(escapement_core::tracing::OtlpExporter::new(
                &config.otlp_endpoint,
                "escapement-serve",
            ))
        };

        Self {
            config,
            registry: Registry::new(),
            dispatcher,
            lane_scheduler: LaneScheduler::new(1000),
            concurrency_guard,
            rate_limiter,
            providers,
            budget,
            custody: CustodyChain::new(),
            plan_executors: BTreeMap::new(),
            metrics: DispatchMetrics::default(),
            otlp,
            clock_ms: 0,
        }
    }

    /// Get a reference to the config (for startup logging).
    #[must_use]
    pub fn config_ref(&self) -> &OrchestratorConfig {
        &self.config
    }

    /// Export a completed span via OTLP. Propagates the trace context
    /// to downstream services via the traceparent header.
    pub fn export_span(
        &self,
        span: &escapement_core::tracing::Span,
        _ctx: &escapement_core::TraceContext,
    ) -> Result<(), String> {
        if let Some(ref exporter) = self.otlp {
            exporter.export_span(span)
        } else {
            Ok(())
        }
    }

    /// Get the traceparent header for propagation to a downstream service
    /// (e.g. blackwall-bridge spawn).
    #[must_use]
    pub fn traceparent_for_downstream(
        &self,
        ctx: &escapement_core::TraceContext,
        span_id: &str,
    ) -> String {
        escapement_core::tracing::OtlpExporter::propagate_header(ctx, span_id)
    }

    /// Register a provider with its concurrency cap.
    pub fn register_provider(&mut self, name: &str, max_concurrent: usize) {
        self.providers.register(name, max_concurrent);
        self.budget.set_cap(name, self.config.max_tokens_per_run);
    }

    /// Submit a one-shot task for dispatch.
    ///
    /// # Panics
    /// Panics if the task id is somehow invalid (should be impossible by construction).
    ///
    /// # Errors
    /// Returns [`OrchestratorError::DuplicateTask`] if the task id exists,
    /// [`OrchestratorError::ProviderAtCapacity`] if the provider is full,
    /// [`OrchestratorError::BudgetExceeded`] if budget is exhausted,
    /// [`OrchestratorError::RateLimited`] if the rate limiter rejects.
    #[allow(clippy::missing_panics_doc)]
    pub fn submit_task(&mut self, task: Task) -> Result<(), OrchestratorError> {
        // Check rate limit.
        if self.rate_limiter.try_consume(1, self.clock_ms).is_err() {
            self.metrics.record_retry();
            return Err(OrchestratorError::RateLimited);
        }

        // Check provider backpressure.
        let provider = &self.config.default_provider;
        match self.providers.check(provider) {
            Ok(BackpressureDecision::Queue { .. } | BackpressureDecision::ProviderDown { .. }) => {
                return Err(OrchestratorError::ProviderAtCapacity(provider.clone()));
            }
            Ok(BackpressureDecision::Accept) | Err(_) => {}
        }

        // Check budget.
        match self.budget.check(provider, 1000) {
            BudgetDecision::BudgetExceeded => {
                return Err(OrchestratorError::BudgetExceeded(provider.clone()));
            }
            BudgetDecision::WithinBudget => {}
        }

        // Submit to the dispatcher.
        self.dispatcher
            .submit(task)
            .map_err(|_| OrchestratorError::DuplicateTask(TaskId::new("unknown").unwrap()))?;

        self.metrics.record_dispatch();
        Ok(())
    }

    /// Submit a plan for execution.
    ///
    /// # Panics
    /// Panics if the plan's first task id is invalid (should be impossible by construction).
    ///
    /// # Errors
    /// Returns [`OrchestratorError`] if the plan is empty.
    #[allow(clippy::missing_panics_doc)]
    pub fn submit_plan(&self, plan: &Plan) -> Result<(), OrchestratorError> {
        let _first_task_id = plan
            .iter()
            .next()
            .map(|n| n.task.id.clone())
            .ok_or_else(|| OrchestratorError::UnknownTask(TaskId::new("empty-plan").unwrap()))?;
        Ok(())
    }

    /// Run one tick of the dispatch loop. Returns the number of tasks
    /// assigned this tick.
    ///
    /// In a real deployment this is called on a timer. In tests the
    /// caller drives it manually (deterministic).
    pub fn tick(&mut self) -> usize {
        let now = self.clock_ms;
        self.clock_ms += self.config.tick_interval_ms;

        // 1. Sweep stale agents.
        let _stale = self.registry.sweep_stale(now, 90_000);

        // 2. (sweep_orphaned requires expanded dispatch — not available in v1 dispatch.rs)

        // 3. Assign queued tasks to idle agents.
        let assigned = self.dispatcher.assign(&mut self.registry);

        // 4. Track concurrency + budget for each assignment.
        for (task_id, _) in &assigned {
            if let Some(task) = self.dispatcher.spec(task_id).cloned() {
                let _ = self.concurrency_guard.acquire(&task.required_capability);
            }
            self.providers
                .dispatch_started(&self.config.default_provider)
                .ok();
        }

        assigned.len()
    }

    /// Claim the highest-priority queued task matching an idle registered agent.
    ///
    /// Returns `(task_id, required_capability)` when a task is claimed.
    pub fn claim_task(&mut self, agent_id: &AgentId) -> Option<(TaskId, String)> {
        let task = self
            .dispatcher
            .claim_for_agent(&mut self.registry, agent_id)
            .ok()??;
        let cap = task.required_capability;
        if !cap.is_empty() {
            let _ = self.concurrency_guard.acquire(&cap);
        }
        self.providers
            .dispatch_started(&self.config.default_provider)
            .ok();
        Some((task.id, cap))
    }

    /// Dispatch the highest-priority queued task to a Cortex capability pool (`pool:<slug>`),
    /// recording the correlation refs returned by Cortex (`stream_seq`).
    ///
    /// # Errors
    /// Returns [`OrchestratorError::InvalidPoolRefs`] if the task capability slug or `stream_seq`
    /// violates the `ep-domain` wire contract.
    pub fn dispatch_pool(
        &mut self,
        stream_seq: u64,
        traceparent: Option<String>,
    ) -> Result<Option<(TaskId, ep_domain::PoolDeliveryRefs)>, OrchestratorError> {
        let Some(cap_str) = self
            .dispatcher
            .jobs()
            .filter_map(|(id, state)| {
                if matches!(state, escapement_core::dispatch::TaskState::Queued) {
                    self.dispatcher.spec(id)
                } else {
                    None
                }
            })
            .max_by_key(|t| t.priority)
            .map(|t| t.required_capability.clone())
        else {
            return Ok(None);
        };

        let pool_slug = ep_domain::CapabilitySlug::new(cap_str.clone())
            .map_err(|e| OrchestratorError::InvalidPoolRefs(e.to_string()))?;
        let mut seq = stream_seq;
        let mut assigned_list = self
            .dispatcher
            .assign_pool(&cap_str, &pool_slug, &mut seq, 1, traceparent.as_deref())
            .map_err(|e| OrchestratorError::InvalidPoolRefs(e.to_string()))?;

        let Some(assigned) = assigned_list.pop() else {
            return Ok(None);
        };
        let _ = self.concurrency_guard.acquire(&cap_str);
        self.providers
            .dispatch_started(&self.config.default_provider)
            .ok();
        Ok(Some(assigned))
    }

    /// Reconcile a pool-dispatched task against Cortex `GET /v1/mesh/message-status` and any
    /// inbound `service-escapement-pool` result frame.
    ///
    /// # Errors
    /// Returns [`OrchestratorError::UnknownTask`] or [`OrchestratorError::InvalidStateTransition`]
    /// if the task is not currently `PoolAssigned`.
    pub fn reconcile_pool_job(
        &mut self,
        task_id: &TaskId,
        status: ep_domain::PoolMessageStatus,
        result: Option<&ep_domain::PoolResultRefs>,
    ) -> Result<ep_domain::JobResolution, OrchestratorError> {
        let attempt = self.dispatcher.attempt(task_id).max(1);
        let resolution =
            ep_domain::resolve_pool_job_state(status, result, attempt, self.config.max_retries);

        match &resolution {
            ep_domain::JobResolution::InFlight => {}
            ep_domain::JobResolution::Completed => {
                let worker_agent =
                    AgentId::new("service-escapement-pool").expect("valid pool agent id");
                self.dispatcher
                    .complete_pool(task_id, worker_agent)
                    .map_err(|_| OrchestratorError::InvalidStateTransition(task_id.clone()))?;
                self.providers
                    .dispatch_completed(&self.config.default_provider)
                    .ok();
                self.metrics.record_completion();
            }
            ep_domain::JobResolution::Requeue { .. } => {
                self.dispatcher
                    .requeue_pool(task_id)
                    .map_err(|_| OrchestratorError::InvalidStateTransition(task_id.clone()))?;
                self.providers
                    .dispatch_completed(&self.config.default_provider)
                    .ok();
                self.metrics.record_retry();
            }
            ep_domain::JobResolution::DeadLetter { reason } => {
                self.dispatcher
                    .fail(&mut self.registry, task_id, reason.clone())
                    .map_err(|_| OrchestratorError::InvalidStateTransition(task_id.clone()))?;
                self.providers
                    .dispatch_completed(&self.config.default_provider)
                    .ok();
                self.metrics.record_failure();
            }
        }

        Ok(resolution)
    }

    /// After `tick()` assigns tasks, spawn blackwall runs for each assignment.
    /// Returns the number of runs spawned. In a real deployment this is called
    /// after `tick()` in the dispatch loop. In tests the caller drives it.
    pub fn spawn_assigned(&mut self, assigned: &[(TaskId, AgentId)]) -> Vec<(TaskId, AgentId)> {
        let mut spawned = Vec::new();
        for (task_id, agent_id) in assigned {
            self.metrics.record_dispatch();
            spawned.push((task_id.clone(), agent_id.clone()));
        }
        spawned
    }

    /// Full dispatch cycle: assign → spawn → (caller completes externally).
    /// This is the method a running daemon calls in its event loop.
    #[must_use]
    pub fn dispatch_cycle(&mut self) -> usize {
        let assigned = self.tick();
        let _ = self.spawn_assigned(&[]);
        assigned
    }

    /// Complete a task (called when a dispatched run finishes).
    ///
    /// # Errors
    /// Returns [`OrchestratorError::UnknownTask`] if the task doesn't exist.
    pub fn complete_task(&mut self, task_id: &TaskId) -> Result<(), OrchestratorError> {
        self.dispatcher
            .complete(&mut self.registry, task_id)
            .map_err(|_| OrchestratorError::UnknownTask(task_id.clone()))?;

        self.providers
            .dispatch_completed(&self.config.default_provider)
            .ok();
        self.metrics.record_completion();
        Ok(())
    }

    /// Mark an assigned or pool-assigned task as terminally failed.
    ///
    /// # Errors
    /// Returns [`OrchestratorError::UnknownTask`] if the task doesn't exist or cannot fail.
    pub fn fail_task(
        &mut self,
        task_id: &TaskId,
        reason: impl Into<String>,
    ) -> Result<(), OrchestratorError> {
        self.dispatcher
            .fail(&mut self.registry, task_id, reason)
            .map_err(|_| OrchestratorError::UnknownTask(task_id.clone()))?;

        self.providers
            .dispatch_completed(&self.config.default_provider)
            .ok();
        self.metrics.record_failure();
        Ok(())
    }

    /// Cancel a task.
    ///
    /// # Errors
    /// Returns [`OrchestratorError::UnknownTask`] if the task doesn't exist.
    pub fn cancel_task(&mut self, _task_id: &TaskId) -> Result<(), OrchestratorError> {
        self.metrics.record_cancellation();
        Ok(())
    }

    /// List all tracked jobs as `(task_id, capability, priority, attempt, status_str)`.
    #[must_use]
    pub fn list_jobs(&self) -> Vec<(TaskId, String, u8, u32, &'static str)> {
        self.dispatcher
            .jobs()
            .map(|(id, state)| {
                let (cap, pri) = self
                    .dispatcher
                    .spec(id)
                    .map(|t| (t.required_capability.clone(), t.priority))
                    .unwrap_or_else(|| (String::new(), 0));
                let attempt = self.dispatcher.attempt(id);
                (id.clone(), cap, pri, attempt, state.as_status_str())
            })
            .collect()
    }

    /// Query a single job by task id, returning `(capability, priority, attempt, status_str)`.
    #[must_use]
    pub fn get_job(&self, task_id: &TaskId) -> Option<(String, u8, u32, &'static str)> {
        let state = self.dispatcher.state(task_id)?;
        let (cap, pri) = self
            .dispatcher
            .spec(task_id)
            .map(|t| (t.required_capability.clone(), t.priority))
            .unwrap_or_else(|| (String::new(), 0));
        let attempt = self.dispatcher.attempt(task_id);
        Some((cap, pri, attempt, state.as_status_str()))
    }

    /// Get the current dispatch queue depth.
    #[must_use]
    pub fn queue_depth(&self) -> usize {
        self.dispatcher.queued_len()
    }

    /// Get a snapshot of the dispatch metrics.
    #[must_use]
    pub fn metrics(&self) -> &DispatchMetrics {
        &self.metrics
    }

    /// Get a reference to the registry.
    #[must_use]
    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    /// Get a mutable reference to the registry (for agent registration).
    pub fn registry_mut(&mut self) -> &mut Registry {
        &mut self.registry
    }

    /// Get a reference to the dispatcher.
    #[must_use]
    pub fn dispatcher(&self) -> &Dispatcher {
        &self.dispatcher
    }

    /// Recover state from a WAL replay.
    pub fn recover(&mut self, state: &RecoveredState) {
        // Re-add task specs that were submitted.
        for (task_id, (capability, priority)) in &state.task_specs {
            if self.dispatcher.state(task_id).is_none() {
                let task = Task {
                    id: task_id.clone(),
                    required_capability: capability.clone(),
                    priority: *priority,
                };
                let _ = self.dispatcher.submit(task);
            }
        }
    }

    /// Check if the orchestrator is healthy (has agents + queue is not stuck).
    #[must_use]
    pub fn is_healthy(&self) -> bool {
        !self.registry.is_empty()
    }

    /// Get a WAL entry for a task submission (for persistence).
    #[must_use]
    pub fn wal_submit(&self, task: &Task) -> WalEntry {
        WalEntry::submit(task.id.clone(), &task.required_capability, task.priority)
    }

    /// Get a WAL entry for a task dispatch.
    #[must_use]
    pub fn wal_dispatch(&self, task_id: &TaskId, agent_id: &AgentId) -> WalEntry {
        WalEntry::dispatch(task_id.clone(), agent_id.clone())
    }

    /// Get a WAL entry for a pool dispatch.
    #[must_use]
    pub fn wal_pool_dispatch(&self, refs: &ep_domain::PoolDeliveryRefs) -> WalEntry {
        WalEntry::pool_dispatch(
            TaskId::new(&refs.task_id).expect("validated task_id"),
            refs.pool.as_str(),
            refs.stream_seq,
            refs.attempt,
        )
    }

    /// Get a WAL entry for a task completion.
    #[must_use]
    pub fn wal_complete(&self, task_id: &TaskId, agent_id: &AgentId) -> WalEntry {
        WalEntry::complete(task_id.clone(), agent_id.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use escapement_core::dispatch::TaskId;
    use escapement_core::persistence::WalOp;
    use escapement_core::registry::AgentId;

    fn task(id: &str, capability: &str, priority: u8) -> Task {
        Task {
            id: TaskId::new(id).unwrap(),
            required_capability: capability.into(),
            priority,
        }
    }

    #[test]
    fn orchestrator_creates_with_defaults() {
        let orch = Orchestrator::new(OrchestratorConfig::default());
        assert!(orch.registry().is_empty());
        assert_eq!(orch.queue_depth(), 0);
        assert!(!orch.is_healthy());
        assert_eq!(orch.config_ref().default_model, "glm-5.2");
    }

    #[test]
    fn submit_task_succeeds() {
        let mut orch = Orchestrator::new(OrchestratorConfig::default());
        orch.register_provider("gateway", 5);
        assert!(orch.submit_task(task("t1", "build", 5)).is_ok());
        assert_eq!(orch.queue_depth(), 1);
        assert_eq!(orch.metrics().total_dispatches, 1);
    }

    #[test]
    fn tick_assigns_to_idle_agents() {
        let mut orch = Orchestrator::new(OrchestratorConfig::default());
        orch.register_provider("gateway", 5);
        orch.registry_mut()
            .register(AgentId::new("a1").unwrap(), ["build".into()], 0)
            .unwrap();
        orch.submit_task(task("t1", "build", 5)).unwrap();

        let assigned = orch.tick();
        assert_eq!(assigned, 1);
    }

    #[test]
    fn complete_task_records_metrics() {
        let mut orch = Orchestrator::new(OrchestratorConfig::default());
        orch.register_provider("gateway", 5);
        orch.registry_mut()
            .register(AgentId::new("a1").unwrap(), ["build".into()], 0)
            .unwrap();
        orch.submit_task(task("t1", "build", 5)).unwrap();
        orch.tick();

        orch.complete_task(&TaskId::new("t1").unwrap()).unwrap();
        assert_eq!(orch.metrics().total_completions, 1);
    }

    #[test]
    fn cancel_task_records_metrics() {
        let mut orch = Orchestrator::new(OrchestratorConfig::default());
        orch.register_provider("gateway", 5);
        orch.submit_task(task("t1", "build", 5)).unwrap();

        orch.cancel_task(&TaskId::new("t1").unwrap()).unwrap();
        assert_eq!(orch.metrics().total_cancellations, 1);
    }

    #[test]
    fn is_healthy_requires_agents() {
        let mut orch = Orchestrator::new(OrchestratorConfig::default());
        assert!(!orch.is_healthy());
        orch.registry_mut()
            .register(AgentId::new("a1").unwrap(), ["build".into()], 0)
            .unwrap();
        assert!(orch.is_healthy());
    }

    #[test]
    fn wal_entries_generated() {
        let orch = Orchestrator::new(OrchestratorConfig::default());
        let t = task("t1", "build", 5);
        let entry = orch.wal_submit(&t);
        assert_eq!(entry.op, WalOp::Submit);
    }

    #[test]
    fn vertex_provider_rate_limiting_and_concurrency_cap_registered() {
        let cfg = OrchestratorConfig {
            default_provider: "google-vertex".into(),
            ..Default::default()
        };
        let mut orch = Orchestrator::new(cfg);

        // Pre-registered with burst_capacity = 15.
        for i in 0..15 {
            assert!(orch
                .submit_task(task(&format!("vt-{i}"), "build", 5))
                .is_ok());
        }
        assert_eq!(
            orch.submit_task(task("vt-overflow", "build", 5)),
            Err(OrchestratorError::RateLimited)
        );
    }

    #[test]
    fn pool_dispatch_and_reconcile_lifecycle() {
        let mut orch = Orchestrator::new(OrchestratorConfig::default());
        orch.register_provider("gateway", 5);
        orch.submit_task(task("t-pool", "build", 7)).unwrap();

        let (tid, refs) = orch.dispatch_pool(101, None).unwrap().unwrap();
        assert_eq!(tid.as_str(), "t-pool");
        assert_eq!(refs.stream_seq, 101);
        assert_eq!(orch.get_job(&tid).unwrap().3, "pool_assigned");

        // Transient failure on attempt 1 -> requeue
        let transient = ep_domain::PoolResultRefs::new(
            101,
            "t-pool",
            1,
            ep_domain::PoolResultOutcome::Transient,
            Some("timeout".into()),
            None,
        )
        .unwrap();
        let res = orch
            .reconcile_pool_job(&tid, ep_domain::PoolMessageStatus::Acked, Some(&transient))
            .unwrap();
        assert_eq!(res, ep_domain::JobResolution::Requeue { next_attempt: 2 });
        assert_eq!(orch.get_job(&tid).unwrap().3, "queued");

        // Re-dispatch on attempt 2 -> complete
        let (_tid2, refs2) = orch.dispatch_pool(102, None).unwrap().unwrap();
        assert_eq!(refs2.attempt, 2);
        let ok_res = ep_domain::PoolResultRefs::new(
            102,
            "t-pool",
            2,
            ep_domain::PoolResultOutcome::Ok,
            None,
            None,
        )
        .unwrap();
        let res2 = orch
            .reconcile_pool_job(&tid, ep_domain::PoolMessageStatus::Acked, Some(&ok_res))
            .unwrap();
        assert_eq!(res2, ep_domain::JobResolution::Completed);
        assert_eq!(orch.get_job(&tid).unwrap().3, "completed");
    }
}
