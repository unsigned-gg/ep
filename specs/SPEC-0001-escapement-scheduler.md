# SPEC-0001: Escapement Scheduler & Deterministic Event Reactor

## 1. Status & Metadata
- **Status:** Approved & Living
- **Authors:** Volva <volva@unsigned.gg>, Sindri <sindri@unsigned.gg>, Mimir <mimir@unsigned.gg>
- **Champion Inspiration:** Carl Lerche, Leslie Lamport, Mara Bos
- **Target Crate:** `ep` (`escapement-core`, `ep-domain`, `escapement-serve`)
- **License:** Apache-2.0 OR MIT / CC0 1.0 Universal

---

## 2. Problem Statement
Autonomous agent swarms require non-blocking, metered, and deterministic task scheduling. In unbounded asynchronous runtimes, unmetered fan-out causes memory exhaustion, cascading retry storms, and unverified state transitions.

`ep` introduces an exact physical horological metaphor: an **escapement mechanism**. Just as a watch escapement meters potential energy into discrete, predictable ticks, `ep` converts raw task graphs into strictly regulated execution ticks with cryptographic custody tracking.

---

## 3. Mathematical Foundations & Operational Invariants

### Invariant 1: Strictly Bounded Queueing & Backpressure
For any execution lane $L_i$ with capacity $C_i$:
$$\text{queued}(L_i) \le C_i \quad \forall t \ge 0$$
When $\text{queued}(L_i) = C_i$, admissions return an immediate deterministic backpressure rejection (`Receipt::Rejected(Backpressure)`), preventing unbounded heap growth.

### Invariant 2: Deterministic Topological Order
Every task plan $P = (V, E)$ is a Directed Acyclic Graph (DAG). If there exists a cycle:
$$\exists v_1, v_2, \dots, v_k \in V \text{ s.t. } (v_k, v_1) \in E \implies \text{Plan::validate}(P) = \text{Err}(\text{CycleDetected})$$
Resolution produces a strictly deterministic topological ordering sorted by `(priority, submit_time, node_id)`.

### Invariant 3: Zero Zombie Custody
Every admitted task $T$ must reside in exactly one custody state:
$$\text{State}(T) \in \{\text{Admitted}, \text{Assigned}(A), \text{Running}(A), \text{Completed}, \text{Failed}, \text{Cancelled}\}$$
Transitions require atomic compare-and-swap handoffs. No task may be unassigned without an active timeout watch.

### Invariant 4: Token-Bucket Rate Metering
Capabilities are metered via leaky token buckets $(R, B)$ where $R$ is refill rate (tokens/sec) and $B$ is burst capacity. Consuming $k$ tokens requires:
$$\text{tokens}(c) \ge k$$
Refills occur monotonically on tick boundaries without floating-point arithmetic.

---

## 4. Architecture & Interface

```
     ┌────────────────────────────────────────────────────────┐
     │                     ep CLI & Daemon                    │
     └───────────────────────────┬────────────────────────────┘
                                 │
                 ┌───────────────┴───────────────┐
                 ▼                               ▼
     ┌───────────────────────┐       ┌───────────────────────┐
     │    escapement-serve   │       │   blackwall-bridge    │
     │   (HTTP/gRPC/OTLP)    │       │   (Isolated Custody)  │
     └───────────┬───────────┘       └───────────┬───────────┘
                 │                               │
                 └───────────────┬───────────────┘
                                 ▼
     ┌────────────────────────────────────────────────────────┐
     │                    escapement-core                     │
     │   • Registry (Heartbeats, sweep, state)               │
     │   • Plan DAG (Kahn's topo-sort, cycle rejection)       │
     │   • Lane Scheduler (Weighted fair queueing)           │
     │   • Metering (Token bucket rate limits)               │
     │   • WAL (Append-only state machine persistence)       │
     └───────────────────────────┬────────────────────────────┘
                                 ▼
     ┌────────────────────────────────────────────────────────┐
     │                       ep-domain                        │
     │   (Zero-I/O Algebraic Data Types, TaskId, PlanNode)   │
     └────────────────────────────────────────────────────────┘
```

---

## 5. Verification & Testing Discipline

1. **Host Throttling (`-j 2`):** All test and build steps run with `-j 2` to prevent CPU and memory exhaustion on multi-agent host `ceres`.
2. **Deterministic Model Checking:** Concurrency and lane fairness properties are validated using deterministic state machine permutation tests.
3. **Miri Cleanliness:** Pure-domain memory conversions and custody handoffs must execute without undefined behavior under `cargo test`.
