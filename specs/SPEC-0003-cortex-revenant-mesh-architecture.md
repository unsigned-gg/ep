---
status: complete
created: 2026-10-15
tags: [cortex, revenant, mesh-pool, custody, scheduler, leanspec]
priority: critical
scope: ep-domain, escapement-core, escapement-serve, blackwall-bridge, ep
depends_on: [SPEC-0001, SPEC-0002]
---

# SPEC-0003: Three-Plane Escapement Architecture (`ep` <-> `cortex` <-> `revenant`)

## Requirements

1. The architecture shall maintain a strict three-plane separation of responsibilities across `ep`, `cortex`, and `revenant`:
   - `ep` owns deterministic tick metering, weighted fair lane queueing, Kahn DAG dependency resolution, stagnation detection, and BLAKE3 custody transitions.
   - `cortex` owns Redis Consumer Group message transport (`workstreams/<slug>`), `claim_timeout_ms` crash recovery, and `/v1/mesh/message-status` reconciliation.
   - `revenant` owns Blackwall OS process isolation (`/v1/runs`), autonomous loop intake (`/v1/escapement/loops`), and OTLP telemetry aggregation.
2. Every capability slug $S$ routed to a Cortex Mesh Pool shall be non-empty, maximum 64 ASCII bytes, and restricted to lowercase ASCII alphanumeric characters, hyphens (`-`), and underscores (`_`):
   $$\forall c \in S,\; c \in [\text{'a'}..\text{'z'}] \cup [\text{'0'}..\text{'9'}] \cup \{\text{'-'}, \text{'\_'}\}$$
3. Every task dispatched to a Cortex Mesh Pool shall transition atomically into `CustodyMode::Pool` (`PoolAssigned`) with immutable delivery references `(pool, message_id, stream_seq, dispatched_at_tick, attempt)` where `stream_seq >= 1` and numeric `refs` fields (`stream_seq`, `attempt`) satisfy $\le 2^{53}$ (`9,007,199,254,740,992`) for exact `pbjson_types::Struct` round-tripping.
4. Pool result reconciliation (`resolve_pool_job_state`) shall be a pure, deterministic state transition in `ep-domain` with zero I/O:
   - An explicit worker result `MSG` (`refs.{stream_seq, task_id, attempt, outcome}`) with `outcome == "ok"` (or `"completed"`) resolves to `JobResolution::Complete`.
   - An explicit worker result `MSG` with `outcome == "transient"` resolves to `JobResolution::RequeueOrFail`.
   - An explicit worker result `MSG` with `outcome == "permanent"` (or `"failed"`) resolves to `JobResolution::Fail`.
   - A Cortex `GET /v1/mesh/message-status?peer_id=<uuid>&pool=<slug>&stream_seq=<n>` query reporting `status == "dead_lettered"` (`PoolMessageStatus::DeadLettered`) resolves to `JobResolution::RequeueOrFail`.
   - A Cortex `message-status` query reporting `"unread"` (`PoolMessageStatus::Unread`), `"pending"` (`PoolMessageStatus::Pending` / in-flight on `member`), or `"acked"` (`PoolMessageStatus::Acked` awaiting worker result `MSG`) resolves to `JobResolution::InFlight`.
5. Pool backpressure shall bound maximum in-flight pool jobs as a pure function of pool consumer count and lane capacity:
   $$\text{max\_inflight}(N_{\text{consumers}}, C_{\text{lane}}, Q_{\text{per\_consumer}}) = \min(C_{\text{lane}}, \max(1, N_{\text{consumers}}) \times \max(1, Q_{\text{per\_consumer}}))$$
6. Terminal job custody records (`Completed`, `Failed`) in `escapement-core::Engine` shall be retained in a bounded FIFO history capped at `MAX_TERMINAL_HISTORY = 1000` entries so `GET /jobs` and `GET /jobs/:id` never return stub responses or grow without bound.

```lean
namespace Escapement.MeshPoolSpec

/-- Cortex /v1/mesh/message-status states (WS-X1 / WS-X2). -/
inductive PoolMessageStatus : Type where
  | unread : PoolMessageStatus
  | pending : Option String -> Nat -> PoolMessageStatus
  | acked : PoolMessageStatus
  | dead_lettered : Nat -> PoolMessageStatus

/-- Deterministic resolution emitted by pure pool reconciliation. -/
inductive JobResolution : Type where
  | in_flight : JobResolution
  | complete : JobResolution
  | fail : JobResolution
  | requeue_or_fail : JobResolution

/-- Pure reconciliation of message status when no explicit worker result has arrived. -/
def reconcile_status (s : PoolMessageStatus) : JobResolution :=
  match s with
  | PoolMessageStatus.unread => JobResolution.in_flight
  | PoolMessageStatus.pending _ _ => JobResolution.in_flight
  | PoolMessageStatus.acked => JobResolution.in_flight
  | PoolMessageStatus.dead_lettered _ => JobResolution.requeue_or_fail

/-- Theorem: Dead-lettered pool messages always trigger deterministic recovery. -/
theorem dead_letter_recovers (deliveries : Nat) :
    reconcile_status (PoolMessageStatus.dead_lettered deliveries) = JobResolution.requeue_or_fail := by
  rfl
```

## Design

### Three-Plane Topology

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                        PLANE 3: REVENANT CONTROL PLANE                       │
│  • Escapement Loop Intake (/v1/escapement/loops)                             │
│  • Blackwall Sandbox Execution (/v1/runs)                                    │
│  • Telemetry & VictoriaMetrics / OTLP Collector                              │
└───────────────┬──────────────────────────────────────────────▲───────────────┘
                │ POST /plans, /tasks                          │ POST /v1/runs
                ▼                                              │
┌──────────────────────────────────────────────────────────────┴───────────────┐
│                        PLANE 1: EP ESCAPEMENT ENGINE                         │
│  • ep-domain: Pure Tick, TokenBucket, CapabilitySlug, resolve_pool_job_state │
│  • escapement-core: Kahn DAG, Weighted Fair Lanes, Custody, Bounded History  │
│  • escapement-serve: HTTP (:8080), Direct Worker API, Pool Reconciler        │
└───────────────┬──────────────────────────────────────────────▲───────────────┘
                │ POST /v1/mesh/route-send                     │ Result MSG +
                │ (to = "workstreams/<slug>")                  │ message-status
                ▼                                              │
┌──────────────────────────────────────────────────────────────┴───────────────┐
│                        PLANE 2: CORTEX MESH TRANSPORT                        │
│  • Redis Stream Consumer Groups (workstreams/<capability>)                   │
│  • Claim Timeout Re-delivery & Dead-Letter Accounting                        │
│  • Sovereign Lane Identity: estate/w4C:p1 (session-agy-ceres-estate-w4c-p1)  │
└──────────────────────────────────────────────────────────────────────────────┘
```

### Custody Dual-Path Matrix

| Custody Mode | Transport Path | Assignment Record | Completion Trigger | Timeout / Crash Recovery |
| :--- | :--- | :--- | :--- | :--- |
| `CustodyMode::DirectWorker` | Direct HTTP (`POST /tasks/claim`) or `blackwall-bridge` (`POST /v1/runs`) | `TaskState::Assigned(AgentId)` | `POST /tasks/complete` or `/v1/runs` response | `Engine::reconcile_orphans` sweeps stale heartbeats |
| `CustodyMode::Pool` | Cortex `POST /v1/mesh/route-send` to `workstreams/<slug>` | `TaskState::PoolAssigned(PoolDeliveryRefs)` | Worker sends result `MSG` to `service-escapement-pool` or calls `POST /tasks/complete` | Cortex consumer group `claim_timeout_ms` + `/v1/mesh/message-status` dead-letter check |

## Tasks

- [x] Establish sovereign lane identity (`estate/w4C:p1`, role `session-agy-ceres-estate-w4c-p1`) and `.saga` team topologies (`.saga/teams.toml`, `.saga/topologies.toml`).
- [x] Implement pure Cortex Mesh Pool types (`CapabilitySlug`, `CustodyMode`, `PoolDeliveryRefs`, `PoolResultRefs`, `PoolMessageStatus`, `JobResolution`, `resolve_pool_job_state`, `max_pool_inflight`) in `ep-domain` (`crates/ep-domain/src/lib.rs`).
- [x] Extend `escapement-core` custody and WAL with `PoolAssigned`, `Failed`, and bounded 1000-entry terminal job history (`crates/escapement-core/src/dispatch.rs`, `crates/escapement-core/src/persistence.rs`).
- [x] Replace `GET /jobs` stub in `escapement-serve` with real active and terminal job inspection (`GET /jobs`, `GET /jobs/:id`) and direct worker lifecycle endpoints (`POST /tasks/{claim,complete,fail}`) (`crates/escapement-serve/src/http.rs`).
- [x] Implement Cortex Mesh Pool dispatcher and reconciler (`crates/escapement-serve/src/pool.rs`).
- [x] Align LiteLLM gateway default model identifier to `"glm-5.2"` in `blackwall-bridge` and `escapement-serve` (`crates/blackwall-bridge/src/lib.rs`, `crates/escapement-serve/src/lib.rs`).
- [x] Implement `ep` operator CLI subcommands (`plan`, `wal`, `lanes`, `mesh`) (`crates/ep/src/main.rs`).

## Verification

1. **Workspace Test Suite (Host Throttled):**
   ```bash
   cargo test -j 2 --workspace
   ```
2. **Strict Clippy Lint Gate:**
   ```bash
   cargo clippy -j 2 --workspace --all-targets -- -D warnings
   ```
3. **Zero-I/O Domain Audit:**
   ```bash
   grep -rnE "std::(fs|net|process)|tokio" crates/ep-domain/src/
   ```
   Expected output: zero matches.

## Notes

- **Author:** Gullveig (`Gullveig (ep/types)`)
- **Coordinated Lanes:** `session-agy-ceres-estate-w3y-p4` (`cerebral-work/cortex`), `session-agy-ceres-estate-w3z-p1` (`cerebral-work/revenant`)
- **License:** `Apache-2.0 OR MIT OR CC0-1.0`
