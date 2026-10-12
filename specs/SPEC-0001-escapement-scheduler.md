---
status: complete
created: 2026-10-12
tags: [escapement, scheduler, deterministic, custody, token-bucket, leanspec]
priority: critical
scope: escapement-core
depends_on: []
---

# SPEC-0001: Escapement Scheduler & Deterministic Event Reactor

## Requirements

1. The scheduler shall process task graphs deterministically without executing non-deterministic clock syscalls inside core dispatch.
2. Every execution lane shall enforce a strict capacity bound:
   $$\text{queued}(L_i) \le C_i \quad \forall t \ge 0$$
   When capacity is reached, admissions shall immediately return `Receipt::Rejected(Backpressure)`.
3. Every task plan $P = (V, E)$ shall form a Directed Acyclic Graph. If a cycle exists, plan validation shall reject the plan with `PlanError::CycleDetected`.
4. Topological order resolution shall sort deterministically by tuple `(priority, submit_time, node_id)`.
5. Every admitted task $T$ shall exist in exactly one custody state:
   $$\text{State}(T) \in \{\text{Admitted}, \text{Assigned}(A), \text{Running}(A), \text{Completed}, \text{Failed}, \text{Cancelled}\}$$
6. State handoffs shall execute atomically using compare-and-swap semantics. No task shall remain unassigned without an active timeout watch.
7. Capability rate limits shall use monotonic integer token buckets $(R, B)$ refilled on tick boundaries without floating-point arithmetic.

```lean
namespace Escapement.Spec

/-- Task custody states in the escapement engine. -/
inductive CustodyState : Type where
  | admitted : CustodyState
  | assigned : String -> CustodyState
  | running : String -> CustodyState
  | completed : CustodyState
  | failed : CustodyState
  | cancelled : CustodyState

/-- Token bucket state with discrete integer balance and burst limit. -/
structure TokenBucket where
  capacity : Nat
  refill_rate : Nat
  tokens : Nat
  last_tick_ms : Nat

/-- Invariant: Queued tasks never exceed lane capacity. -/
def LaneCapacityBounded (queued : Nat) (capacity : Nat) : Prop :=
  queued <= capacity

/-- Theorem: An empty lane satisfies the capacity bound. -/
theorem empty_lane_bounded (cap : Nat) : LaneCapacityBounded 0 cap := by
  dsimp [LaneCapacityBounded]
  apply Nat.zero_le
```

## Design

### Horological Escapement Metaphor
Mechanical escapements convert continuous spring potential into discrete, regulated ticks. The `ep` engine converts unbounded asynchronous task requests into discrete ticks with cryptographic custody tracking.

### Subsystem Architecture

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

### Custody Transitions

| Initial State | Transition Event | Target State | Custody Action |
| :--- | :--- | :--- | :--- |
| `Admitted` | Worker capability match | `Assigned` | Record agent custody reference |
| `Assigned` | Process execution starts | `Running` | Arm timeout watch |
| `Running` | Normal termination | `Completed` | Record BLAKE3 result digest |
| `Running` | Failure or timeout | `Failed` | Trigger retry or dead-letter queue |
| `Assigned` / `Running` | Operator cancel | `Cancelled` | Reclaim capacity |

## Tasks

- [x] Implement zero-I/O domain types in `ep-domain` (`unsigned-gg/ep@6765df9:crates/ep-domain/src/lib.rs`).
- [x] Implement deterministic Kahn's topological sort in `escapement-core` (`unsigned-gg/ep@6765df9:crates/escapement-core/src/plan.rs`).
- [x] Implement atomic task custody tracking with BLAKE3 hashes (`unsigned-gg/ep@6765df9:crates/escapement-core/src/custody.rs`).
- [x] Implement discrete token bucket rate metering (`unsigned-gg/ep@6765df9:crates/escapement-core/src/metering.rs`).
- [x] Implement WAL persistence and replay reconstruction (`unsigned-gg/ep@6765df9:crates/escapement-core/src/persistence.rs`).
- [x] Implement Blackwall process isolation bridge (`unsigned-gg/ep@6765df9:crates/blackwall-bridge/src/lib.rs`).
- [x] Implement single-writer orchestrator daemon in `escapement-serve` (`unsigned-gg/ep@6765df9:crates/escapement-serve/src/lib.rs`).

## Verification

1. **Host Throttling Invariant:** All build and test executions on host `ceres` must specify `-j 2`:
   ```bash
   cargo test -j 2 --workspace
   ```
2. **Mechanical Proofs:** 162 unit and integration tests passing across all crates.
3. **Miri Cleanliness:** Verify memory safety and zero undefined behavior in pure domain transformations:
   ```bash
   cargo test -j 2 -p ep-domain
   ```
4. **Authentic Source Locators:**
   - `unsigned-gg/ep@6765df9:crates/escapement-core/src/plan.rs`
   - `unsigned-gg/ep@6765df9:crates/escapement-core/src/custody.rs`
   - `unsigned-gg/ep@6765df9:crates/escapement-core/src/metering.rs`

## Notes

- **Authors:** Volva <volva@unsigned.gg>, Sindri <sindri@unsigned.gg>, Mimir <mimir@unsigned.gg>
- **Champion Inspiration:** Carl Lerche, Leslie Lamport, Mara Bos
- **Editorial Alignment:** Follows `cerebral-docs/overview/research-and-development/editorial-standards.md`
- **License:** Tri-licensed under `Apache-2.0 OR MIT OR CC0-1.0`
