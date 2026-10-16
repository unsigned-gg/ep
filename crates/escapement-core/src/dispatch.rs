//! Task dispatch: priority queue + capability-matched assignment against the
//! registry.

use std::collections::{BTreeMap, VecDeque};
use std::fmt;

use crate::registry::{AgentId, AgentState, Registry, RegistryError};

/// Maximum number of terminal (`Completed` or `Failed`) tasks retained in memory
/// before the oldest terminal entry is evicted.
pub const MAX_TERMINAL_HISTORY: usize = 1000;

/// Unique task identifier. Non-empty, caller-assigned.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskId(String);

impl TaskId {
    /// # Errors
    /// Returns [`DispatchError::InvalidTaskId`] if `id` is empty or
    /// whitespace-only.
    pub fn new(id: impl Into<String>) -> Result<Self, DispatchError> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(DispatchError::InvalidTaskId);
        }
        Ok(Self(id))
    }

    /// Return the task id as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A dispatchable task with a capability requirement and priority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    /// The unique task id.
    pub id: TaskId,
    /// The capability an assigned agent must advertise.
    pub required_capability: String,
    /// Higher dispatches first. Ties break on enqueue order (FIFO).
    pub priority: u8,
}

/// Lifecycle state of a submitted task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskState {
    /// Waiting in the queue for an agent.
    Queued,
    /// Assigned to the given agent.
    Assigned(AgentId),
    /// Assigned to a Cortex mesh pool (`workstreams/<slug>`).
    PoolAssigned(ep_domain::PoolDeliveryRefs),
    /// Completed by the given agent.
    Completed(AgentId),
    /// Failed with a terminal reason.
    Failed(String),
}

impl TaskState {
    /// Return a machine-readable status label for HTTP and CLI inspection.
    #[must_use]
    pub const fn as_status_str(&self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Assigned(_) => "assigned",
            Self::PoolAssigned(_) => "pool_assigned",
            Self::Completed(_) => "completed",
            Self::Failed(_) => "failed",
        }
    }
}

/// Errors from dispatch operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchError {
    /// The task id was empty or whitespace-only.
    InvalidTaskId,
    /// The task id was already submitted.
    DuplicateTask(TaskId),
    /// The task id is not known to the dispatcher.
    UnknownTask(TaskId),
    /// The task is not in an assigned state.
    TaskNotAssigned(TaskId),
    /// An underlying registry error.
    Registry(RegistryError),
    /// A domain invariant failed during pool assignment.
    Domain(ep_domain::DomainError),
}

impl fmt::Display for DispatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTaskId => write!(f, "task id must be non-empty"),
            Self::DuplicateTask(id) => write!(f, "task already submitted: {id}"),
            Self::UnknownTask(id) => write!(f, "unknown task: {id}"),
            Self::TaskNotAssigned(id) => write!(f, "task not assigned: {id}"),
            Self::Registry(e) => write!(f, "registry error: {e}"),
            Self::Domain(e) => write!(f, "domain error: {e}"),
        }
    }
}

impl std::error::Error for DispatchError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Registry(e) => Some(e),
            Self::Domain(e) => Some(e),
            _ => None,
        }
    }
}

impl From<RegistryError> for DispatchError {
    fn from(e: RegistryError) -> Self {
        Self::Registry(e)
    }
}

impl From<ep_domain::DomainError> for DispatchError {
    fn from(e: ep_domain::DomainError) -> Self {
        Self::Domain(e)
    }
}

#[derive(Debug)]
struct QueuedTask {
    task: Task,
    seq: u64,
}

/// Dispatcher over a priority queue of tasks. Assignment is deterministic:
/// highest priority first, FIFO within a priority, lowest agent id among
/// idle capable agents.
#[derive(Debug, Default)]
pub struct Dispatcher {
    queue: Vec<QueuedTask>,
    states: BTreeMap<TaskId, TaskState>,
    specs: BTreeMap<TaskId, Task>,
    attempts: BTreeMap<TaskId, u32>,
    terminal_history: VecDeque<TaskId>,
    next_seq: u64,
}

impl Dispatcher {
    /// Create an empty dispatcher.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Enqueue a task.
    ///
    /// # Errors
    /// Returns [`DispatchError::DuplicateTask`] if the task id was already
    /// submitted (queued, assigned, or completed).
    pub fn submit(&mut self, task: Task) -> Result<(), DispatchError> {
        if self.states.contains_key(&task.id) {
            return Err(DispatchError::DuplicateTask(task.id));
        }
        self.states.insert(task.id.clone(), TaskState::Queued);
        self.specs.insert(task.id.clone(), task.clone());
        self.attempts.insert(task.id.clone(), 0);
        self.queue.push(QueuedTask {
            task,
            seq: self.next_seq,
        });
        self.next_seq += 1;
        Ok(())
    }

    /// Assign as many queued tasks as possible to idle capable agents.
    /// Returns the `(task, agent)` pairs assigned this call. Tasks with no
    /// eligible agent remain queued.
    pub fn assign(&mut self, registry: &mut Registry) -> Vec<(TaskId, AgentId)> {
        // Highest priority first; FIFO within a priority.
        self.queue.sort_by(|a, b| {
            b.task
                .priority
                .cmp(&a.task.priority)
                .then(a.seq.cmp(&b.seq))
        });

        let mut assigned = Vec::new();
        let mut remaining = Vec::new();

        for queued in self.queue.drain(..) {
            match registry.claim_idle_with_capability(&queued.task.required_capability) {
                Some(agent_id) => {
                    *self.attempts.entry(queued.task.id.clone()).or_insert(0) += 1;
                    self.states.insert(
                        queued.task.id.clone(),
                        TaskState::Assigned(agent_id.clone()),
                    );
                    assigned.push((queued.task.id, agent_id));
                }
                None => remaining.push(queued),
            }
        }

        self.queue = remaining;
        assigned
    }

    /// Assign queued tasks matching `capability` to a Cortex mesh pool (`workstreams/<slug>`),
    /// up to `limit` tasks.
    ///
    /// # Errors
    /// Returns [`DispatchError::Domain`] if `next_stream_seq` exceeds `MAX_SAFE_STRUCT_INT`.
    pub fn assign_pool(
        &mut self,
        capability: &str,
        pool_slug: &ep_domain::CapabilitySlug,
        next_stream_seq: &mut u64,
        limit: usize,
        traceparent: Option<&str>,
    ) -> Result<Vec<(TaskId, ep_domain::PoolDeliveryRefs)>, DispatchError> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        self.queue.sort_by(|a, b| {
            b.task
                .priority
                .cmp(&a.task.priority)
                .then(a.seq.cmp(&b.seq))
        });

        let mut assigned = Vec::new();
        let mut remaining = Vec::new();

        for queued in self.queue.drain(..) {
            if assigned.len() < limit && queued.task.required_capability == capability {
                let attempt = {
                    let entry = self.attempts.entry(queued.task.id.clone()).or_insert(0);
                    *entry += 1;
                    *entry
                };
                let seq = *next_stream_seq;
                *next_stream_seq = next_stream_seq.saturating_add(1);
                let refs = ep_domain::PoolDeliveryRefs::new(
                    pool_slug.clone(),
                    seq,
                    queued.task.id.as_str(),
                    attempt,
                    traceparent.map(str::to_string),
                )?;
                self.states.insert(
                    queued.task.id.clone(),
                    TaskState::PoolAssigned(refs.clone()),
                );
                assigned.push((queued.task.id, refs));
            } else {
                remaining.push(queued);
            }
        }

        self.queue = remaining;
        Ok(assigned)
    }

    /// Claim the highest-priority queued task that matches one of the idle
    /// `agent_id`'s capabilities.
    ///
    /// # Errors
    /// Returns [`DispatchError::Registry`] if the agent is not registered.
    pub fn claim_for_agent(
        &mut self,
        registry: &mut Registry,
        agent_id: &AgentId,
    ) -> Result<Option<Task>, DispatchError> {
        let record = registry
            .get(agent_id)
            .ok_or_else(|| RegistryError::UnknownAgent(agent_id.clone()))?;
        if record.state != AgentState::Idle {
            return Ok(None);
        }
        let caps = record.capabilities.clone();

        self.queue.sort_by(|a, b| {
            b.task
                .priority
                .cmp(&a.task.priority)
                .then(a.seq.cmp(&b.seq))
        });

        if let Some(pos) = self
            .queue
            .iter()
            .position(|q| caps.contains(&q.task.required_capability))
        {
            let queued = self.queue.remove(pos);
            registry.set_state(agent_id, AgentState::Busy)?;
            *self.attempts.entry(queued.task.id.clone()).or_insert(0) += 1;
            self.states.insert(
                queued.task.id.clone(),
                TaskState::Assigned(agent_id.clone()),
            );
            Ok(Some(queued.task))
        } else {
            Ok(None)
        }
    }

    /// Mark an assigned task completed and return its agent to `Idle`.
    ///
    /// # Errors
    /// Returns [`DispatchError::UnknownTask`] for a never-submitted id and
    /// [`DispatchError::TaskNotAssigned`] for a queued or already-completed
    /// task.
    pub fn complete(
        &mut self,
        registry: &mut Registry,
        task_id: &TaskId,
    ) -> Result<AgentId, DispatchError> {
        let state = self
            .states
            .get(task_id)
            .ok_or_else(|| DispatchError::UnknownTask(task_id.clone()))?;
        let TaskState::Assigned(agent_id) = state.clone() else {
            return Err(DispatchError::TaskNotAssigned(task_id.clone()));
        };
        registry.set_state(&agent_id, AgentState::Idle)?;
        self.states
            .insert(task_id.clone(), TaskState::Completed(agent_id.clone()));
        self.record_terminal(task_id.clone());
        Ok(agent_id)
    }

    /// Mark a pool-assigned task completed by `worker_agent`.
    ///
    /// # Errors
    /// Returns [`DispatchError::UnknownTask`] if the task does not exist or
    /// [`DispatchError::TaskNotAssigned`] if it is not in [`TaskState::PoolAssigned`].
    pub fn complete_pool(
        &mut self,
        task_id: &TaskId,
        worker_agent: AgentId,
    ) -> Result<ep_domain::PoolDeliveryRefs, DispatchError> {
        let state = self
            .states
            .get(task_id)
            .ok_or_else(|| DispatchError::UnknownTask(task_id.clone()))?;
        let TaskState::PoolAssigned(refs) = state.clone() else {
            return Err(DispatchError::TaskNotAssigned(task_id.clone()));
        };
        self.states
            .insert(task_id.clone(), TaskState::Completed(worker_agent));
        self.record_terminal(task_id.clone());
        Ok(refs)
    }

    /// Requeue a pool-assigned task back into the priority queue (for transient
    /// worker failures or Cortex dead-lettering within retry limits).
    ///
    /// # Errors
    /// Returns [`DispatchError::UnknownTask`] or [`DispatchError::TaskNotAssigned`].
    pub fn requeue_pool(&mut self, task_id: &TaskId) -> Result<u32, DispatchError> {
        let state = self
            .states
            .get(task_id)
            .ok_or_else(|| DispatchError::UnknownTask(task_id.clone()))?;
        let TaskState::PoolAssigned(refs) = state.clone() else {
            return Err(DispatchError::TaskNotAssigned(task_id.clone()));
        };
        let task = self
            .specs
            .get(task_id)
            .cloned()
            .ok_or_else(|| DispatchError::UnknownTask(task_id.clone()))?;
        self.states.insert(task_id.clone(), TaskState::Queued);
        self.queue.push(QueuedTask {
            task,
            seq: self.next_seq,
        });
        self.next_seq += 1;
        Ok(refs.attempt)
    }

    /// Mark an assigned or pool-assigned task as failed with `reason`.
    /// If the task was assigned to a registered agent, that agent is returned to `Idle`.
    ///
    /// # Errors
    /// Returns [`DispatchError::UnknownTask`] or [`DispatchError::TaskNotAssigned`].
    pub fn fail(
        &mut self,
        registry: &mut Registry,
        task_id: &TaskId,
        reason: impl Into<String>,
    ) -> Result<(), DispatchError> {
        let state = self
            .states
            .get(task_id)
            .ok_or_else(|| DispatchError::UnknownTask(task_id.clone()))?
            .clone();
        match state {
            TaskState::Assigned(agent_id) => {
                registry.set_state(&agent_id, AgentState::Idle)?;
            }
            TaskState::PoolAssigned(_) => {}
            TaskState::Queued | TaskState::Completed(_) | TaskState::Failed(_) => {
                return Err(DispatchError::TaskNotAssigned(task_id.clone()));
            }
        }
        self.states
            .insert(task_id.clone(), TaskState::Failed(reason.into()));
        self.record_terminal(task_id.clone());
        Ok(())
    }

    fn record_terminal(&mut self, task_id: TaskId) {
        self.terminal_history.push_back(task_id);
        while self.terminal_history.len() > MAX_TERMINAL_HISTORY {
            if let Some(evicted) = self.terminal_history.pop_front() {
                self.states.remove(&evicted);
                self.specs.remove(&evicted);
                self.attempts.remove(&evicted);
            }
        }
    }

    /// Get the current state of a task.
    #[must_use]
    pub fn state(&self, task_id: &TaskId) -> Option<&TaskState> {
        self.states.get(task_id)
    }

    /// Get the original task specification for a task id.
    #[must_use]
    pub fn spec(&self, task_id: &TaskId) -> Option<&Task> {
        self.specs.get(task_id)
    }

    /// Get the number of dispatch attempts for a task id.
    #[must_use]
    pub fn attempt(&self, task_id: &TaskId) -> u32 {
        self.attempts.get(task_id).copied().unwrap_or(0)
    }

    /// Iterate over all known tasks and their states in deterministic `TaskId` order.
    pub fn jobs(&self) -> impl Iterator<Item = (&TaskId, &TaskState)> {
        self.states.iter()
    }

    /// Number of tasks still waiting in the queue.
    #[must_use]
    pub fn queued_len(&self) -> usize {
        self.queue.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::AgentId;

    fn agent(s: &str) -> AgentId {
        AgentId::new(s).unwrap()
    }

    fn task(id: &str, capability: &str, priority: u8) -> Task {
        Task {
            id: TaskId::new(id).unwrap(),
            required_capability: capability.into(),
            priority,
        }
    }

    fn registry_with(agents: &[(&str, &str)]) -> Registry {
        let mut reg = Registry::new();
        for (id, capability) in agents {
            reg.register(agent(id), [(*capability).into()], 0).unwrap();
        }
        reg
    }

    #[test]
    fn rejects_duplicate_task() {
        let mut d = Dispatcher::new();
        d.submit(task("t1", "build", 0)).unwrap();
        assert_eq!(
            d.submit(task("t1", "build", 5)),
            Err(DispatchError::DuplicateTask(TaskId::new("t1").unwrap()))
        );
    }

    #[test]
    fn assigns_highest_priority_first() {
        let mut reg = registry_with(&[("a", "build")]);
        let mut d = Dispatcher::new();
        d.submit(task("low", "build", 1)).unwrap();
        d.submit(task("high", "build", 9)).unwrap();

        let assigned = d.assign(&mut reg);
        assert_eq!(assigned, vec![(TaskId::new("high").unwrap(), agent("a"))]);
        assert_eq!(d.queued_len(), 1); // "low" waits — only one agent.
    }

    #[test]
    fn fifo_within_same_priority() {
        let mut reg = registry_with(&[("a", "build")]);
        let mut d = Dispatcher::new();
        d.submit(task("first", "build", 5)).unwrap();
        d.submit(task("second", "build", 5)).unwrap();

        let assigned = d.assign(&mut reg);
        assert_eq!(assigned, vec![(TaskId::new("first").unwrap(), agent("a"))]);
    }

    #[test]
    fn no_capable_agent_leaves_task_queued() {
        let mut reg = registry_with(&[("a", "review")]);
        let mut d = Dispatcher::new();
        d.submit(task("t1", "build", 5)).unwrap();

        assert!(d.assign(&mut reg).is_empty());
        assert_eq!(d.queued_len(), 1);
        assert_eq!(
            d.state(&TaskId::new("t1").unwrap()),
            Some(&TaskState::Queued)
        );
    }

    #[test]
    fn completion_frees_agent_for_next_task() {
        let mut reg = registry_with(&[("a", "build")]);
        let mut d = Dispatcher::new();
        d.submit(task("t1", "build", 5)).unwrap();
        d.submit(task("t2", "build", 5)).unwrap();

        let first = d.assign(&mut reg);
        assert_eq!(first.len(), 1);
        assert!(d.assign(&mut reg).is_empty()); // agent busy

        let freed = d.complete(&mut reg, &TaskId::new("t1").unwrap()).unwrap();
        assert_eq!(freed, agent("a"));

        let second = d.assign(&mut reg);
        assert_eq!(second, vec![(TaskId::new("t2").unwrap(), agent("a"))]);
    }

    #[test]
    fn complete_on_queued_task_errors() {
        let mut reg = registry_with(&[]);
        let mut d = Dispatcher::new();
        d.submit(task("t1", "build", 5)).unwrap();
        assert_eq!(
            d.complete(&mut reg, &TaskId::new("t1").unwrap()),
            Err(DispatchError::TaskNotAssigned(TaskId::new("t1").unwrap()))
        );
    }

    #[test]
    fn deterministic_agent_tie_break_by_id() {
        let mut reg = registry_with(&[("b", "build"), ("a", "build")]);
        let mut d = Dispatcher::new();
        d.submit(task("t1", "build", 5)).unwrap();

        let assigned = d.assign(&mut reg);
        assert_eq!(assigned, vec![(TaskId::new("t1").unwrap(), agent("a"))]);
    }

    #[test]
    fn pool_assignment_requeue_and_completion() {
        let mut d = Dispatcher::new();
        d.submit(task("t1", "rust_builder", 7)).unwrap();
        let slug = ep_domain::CapabilitySlug::new("rust_builder").unwrap();
        let mut seq = 10;

        let assigned = d
            .assign_pool("rust_builder", &slug, &mut seq, 2, Some("00-abc-def-01"))
            .unwrap();
        assert_eq!(assigned.len(), 1);
        assert_eq!(assigned[0].1.stream_seq, 10);
        assert_eq!(assigned[0].1.attempt, 1);
        assert_eq!(seq, 11);

        // Requeue on transient failure and re-assign -> attempt increments to 2.
        let prev_attempt = d.requeue_pool(&TaskId::new("t1").unwrap()).unwrap();
        assert_eq!(prev_attempt, 1);
        assert_eq!(d.queued_len(), 1);

        let reassigned = d
            .assign_pool("rust_builder", &slug, &mut seq, 2, None)
            .unwrap();
        assert_eq!(reassigned[0].1.attempt, 2);
        assert_eq!(reassigned[0].1.stream_seq, 11);

        // Complete pool task.
        let refs = d
            .complete_pool(&TaskId::new("t1").unwrap(), agent("pool-worker-1"))
            .unwrap();
        assert_eq!(refs.attempt, 2);
        assert_eq!(
            d.state(&TaskId::new("t1").unwrap()),
            Some(&TaskState::Completed(agent("pool-worker-1")))
        );
    }

    #[test]
    fn terminal_history_evicts_past_max_limit() {
        let mut reg = registry_with(&[("a", "build")]);
        let mut d = Dispatcher::new();

        for i in 0..=MAX_TERMINAL_HISTORY {
            let id = format!("task-{i:04}");
            d.submit(task(&id, "build", 5)).unwrap();
            let assigned = d.assign(&mut reg);
            assert_eq!(assigned.len(), 1);
            d.complete(&mut reg, &TaskId::new(&id).unwrap()).unwrap();
        }

        // task-0000 should be evicted after 1001 terminal completions.
        assert!(d.state(&TaskId::new("task-0000").unwrap()).is_none());
        assert!(d.state(&TaskId::new("task-0001").unwrap()).is_some());
        assert_eq!(d.jobs().count(), MAX_TERMINAL_HISTORY);
    }
}
