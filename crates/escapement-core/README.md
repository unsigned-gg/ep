# escapement-core (Deterministic Dispatch Engine)

Deterministic task scheduling, capability matching, custody tracking, and event reactor execution in autonomous agent constellations.

Part of [unsigned.gg](https://unsigned.gg).

| Attribute | Specification |
|---|---|
| Status | Functional, 151 unit tests passing |
| Roadmap Phase | Phase 1 (Core Engine Foundation) |
| Layer | Layer 1 (Deterministic Core Dispatch Engine) |
| Depends On | `ep-domain` |
| Invariants | [`SPEC-0001 §3 Invariants 1–4`](../../specs/SPEC-0001-escapement-scheduler.md) |

## Architecture & Subsystems

```
                                 ┌─────────────────────────┐
                                 │     Registry & Presence │
                                 └────────────┬────────────┘
                                              │
                      ┌───────────────────────┼───────────────────────┐
                      ▼                       ▼                       ▼
           ┌─────────────────────┐ ┌─────────────────────┐ ┌─────────────────────┐
           │      Plan (DAG)     │ │    Custody Chain    │ │ Priority Lanes &    │
           │  Topological Order  │ │ Atomic CAS Handoffs │ │ Bounded Dispatch    │
           └──────────┬──────────┘ └──────────┬──────────┘ └──────────┬──────────┘
                      │                       │                       │
                      └───────────────────────┼───────────────────────┘
                                              ▼
                                 ┌─────────────────────────┐
                                 │  Token Metering & WAL   │
                                 │  Persistence Replay     │
                                 └─────────────────────────┘
```

The engine separates responsibilities into modular subsystems:

| Module | Description |
| :--- | :--- |
| **`backpressure`** | Provider health tracking, capacity checks, and immediate backpressure rejections. |
| **`budget`** | Per-run token consumption tracking and maximum token limit enforcement. |
| **`custody`** | Cryptographic custody tracking with immutable dispatch records and BLAKE3 hashes. |
| **`dispatch`** | Priority task queues, execution state machines, and task lifecycle tracking. |
| **`lanes`** | Multi-lane priority scheduler maintaining bounded execution queues. |
| **`merge`** | Changeset conflict detection and branch merging logic. |
| **`metering`** | Integer token-bucket rate limiters and concurrency guards. |
| **`persistence`** | Write-Ahead Log (WAL) operations, replay engine, and state reconstruction. |
| **`plan`** | Directed Acyclic Graph (DAG) plan validation, cycle rejection, and topological ordering. |
| **`presence`** | Peer presence tracking, agent rosters, and heartbeats. |
| **`protocol`** | Typed admission decisions, plan submission formats, and settlement receipts. |
| **`registry`** | Agent identity records, declared capabilities, and runtime availability. |
| **`resilience`** | Bounded retry policies with deterministic backoff. |
| **`telemetry` & `tracing`** | OpenTelemetry metrics and distributed span collection. |
| **`waker`** | Scale-to-zero event waking primitives. |

## Custody State Machine

Tasks progress through strict, verifiable custody states:

| Current State | Event | Next State | Description |
| :--- | :--- | :--- | :--- |
| **`Admitted`** | Assign agent | **`Assigned`** | Lane capacity verified; agent capability matched. |
| **`Assigned`** | Worker starts | **`Running`** | Custody transferred to worker; timeout watch armed. |
| **`Running`** | Task succeeds | **`Completed`** | Result BLAKE3 hash recorded; custody released. |
| **`Running`** | Error occurred | **`Failed`** | Error categorized; retry policy evaluated. |
| **`Assigned` / `Running`** | Cancellation | **`Cancelled`** | Cancellation requested; resources reclaimed. |

## Architectural Invariants

1. **Deterministic Time:** Callers supply time as integer milliseconds. The core engine never invokes `std::time::Instant::now()` directly in scheduling logic.
2. **Strict Queue Bounds:** Lanes reject admissions when capacity reaches limit, returning `Receipt::Rejected(Backpressure)`.
3. **Cycle Freedom:** Plan submission validates absence of cycles using Depth-First Search. Cyclic plans fail with `PlanError::CycleDetected`.
4. **Zero Zombie Tasks:** Every task transition updates custody atomically. No task remains unassigned without an active timeout watch.

## Verification

Run all core engine unit tests under host throttling:

```bash
cargo test -j 2 -p escapement-core
```

## License

Tri-licensed under `Apache-2.0 OR MIT OR CC0-1.0`.
