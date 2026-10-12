# SPEC-0002: Escapement Protocol Roadmap & Evolution Horizons

## 1. Status & Metadata
- **Status:** Approved & Living
- **Authors:** Gullveig <gullveig@types.unsigned.gg>, Mimir <mimir@unsigned.gg>, Volva <volva@unsigned.gg>
- **Champion Inspiration:** Oleg Kiselyov, Leslie Lamport, Carl Lerche
- **Target Crate:** `ep` (`escapement-core`, `ep-domain`, `blackwall-bridge`, `escapement-serve`, `ep`)
- **License:** Apache-2.0 OR MIT / CC0 1.0 Universal
- **Lineage:** Follows `SPEC-0001-escapement-scheduler.md`

---

## 2. Problem Statement & Scope
Autonomous agent swarms require predictable execution horizons. Unmetered task dispatch causes memory exhaustion, cascading retries, and opaque state transitions.

`ep` supplies metered, deterministic dispatch through zero-dependency Rust primitives. This document defines the development roadmap for `ep` across five architectural phases:
1. Core Dispatch Engine (Phase 1 — Shipped).
2. Unified Command-Line Interface (Phase 2 — In Progress).
3. Distributed Scale-to-Zero Integration (Phase 3 — Planned).
4. Formal Verification & Algebraic Lattices (Phase 4 — Planned).
5. Ecosystem Federation & Protocol Stabilization (Phase 5 — Planned).

---

## 3. Phased Roadmap Specifications

### Phase 1: Core Dispatch Engine (Shipped / Foundation)
Phase 1 established the deterministic dispatch foundation. All Phase 1 modules compile with zero external dependencies in `escapement-core`.

- **Deterministic DAG Resolver (`plan.rs`):** Implements Kahn's topological sort. Uses `BTreeSet` tie-breaking for deterministic node ordering. Detects cycles and missing dependencies.
- **Atomic Task Lifecycle (`dispatch.rs`):** Implements a nine-state lifecycle (`Admitted`, `Queued`, `Assigned`, `Running`, `Completed`, `Failed`, `Cancelled`, `TimedOut`, `DeadLettered`).
- **Cryptographic Custody Chains (`custody.rs`):** Tracks content-addressed BLAKE3 provenance chains from root parent tasks to execution outputs.
- **Typed Wire Protocol (`protocol.rs`):** Rejects untyped JSON payloads. Enforces version negotiation (`PROTOCOL_VERSION = "0.1"`) across `PlanSubmission`, `DispatchReceipt`, and `SettleReceipt`.
- **Metered Fair Queuing (`metering.rs`, `lanes.rs`):** Implements leaky token buckets without floating-point arithmetic. Schedules tasks via weighted fair queueing lanes.
- **Append-Only Persistence (`persistence.rs`):** Writes state mutations to a Write-Ahead Log (WAL). Reconstructs dispatcher state upon restart.
- **Containment Bridge (`blackwall-bridge`):** Adapts dispatch jobs to Landlock-confined blackwall sandboxes.
- **Daemon Prototype (`escapement-serve`):** Provides HTTP endpoints (`/healthz`, `/dispatch`, `/jobs`) and OTLP trace context propagation.

### Phase 2: Unified Command-Line Interface & Ergonomics (In Progress)
Phase 2 turns the `crates/ep` binary into a standalone operator tool. The CLI allows offline verification, plan inspection, and WAL auditing.

- **`ep plan validate <file>`:** Parses and checks task graphs for cycles, missing dependencies, and priority validity.
- **`ep plan render <file>`:** Generates topological schedules and prints execution diagrams in ASCII or Mermaid format.
- **`ep wal inspect <log>`:** Decodes WAL records. Verifies operation sequence continuity and checksums.
- **`ep wal replay <log>`:** Simulates recovery in memory. Displays final task states without starting network listeners.
- **`ep lanes status`:** Displays lane queue depths, token capacities, and scheduling weights.
- **Ergonomic Output Modes:** Supports human-readable terminal output and structured JSON output for scripts.

### Phase 3: Distributed Scale-to-Zero & Waker Integration (Planned)
Phase 3 integrates `ep` with scale-to-zero infrastructure to support idle agent fleets.

- **Actuator Integration (`waker.rs`):** Coordinates with `unsigned-gg/waker` (`portolan`) TCP wake proxies.
- **Synchronous Wake Primitives:** Implements `wake_and_wait(target)` with bounded timeouts (maximum 120 seconds).
- **Health-Gated Dispatch:** Prevents task assignment when target pods report restart backoff or failing probes.
- **Multi-Node Topology:** Distributes lane queues across cluster nodes while maintaining deterministic custody records.

### Phase 4: Formal Verification & Algebraic State Models (Planned)
Phase 4 formalizes type-level contracts and verifies concurrent safety.

- **Zero-Allocation Lattices (`merge.rs`, `ep-domain`):** Provides algebraic conflict-free replicated data types (CRDTs) for merging parallel task states.
- **Type-State Lifecycle Constraints:** Encodes valid task states in Rust types to prevent invalid transitions at compile time.
- **Model Checking:**
  - Validates lock-free queues and atomic state transitions using Stateright or Loom.
  - Validates graph resolution using property-based testing (`proptest`) over arbitrary cyclic and acyclic topologies.
- **Undefined Behavior Auditing:** Enforces zero undefined behavior under Miri across domain and core crates.

### Phase 5: Ecosystem Federation & Protocol Stabilization (Planned)
Phase 5 stabilizes external interfaces for wide open-source distribution.

- **Protocol v1.0 Freeze:** Freezes the wire protocol to guarantee backwards compatibility.
- **Crates.io Publication:** Publishes `escapement-core` as a standalone, zero-dependency crate.
- **Edge Gateway:** Deploys Cloudflare Worker edge filters for initial payload validation.
- **Mesh Telemetry:** Standardizes OTLP trace and metric export across the entire agent constellation.

---

## 4. Architectural Invariants & Governance

> [!IMPORTANT]
> All pull requests and code modifications must satisfy the following architectural invariants.

1. **Zero External Dependencies in Core:** The `escapement-core` crate must maintain an empty `[dependencies]` table.
2. **Deterministic Time Execution:** All core routines must accept time as caller-supplied millisecond timestamps. Never call host clock functions in core logic.
3. **Controlled Language Precision:** All documentation, error messages, and rustdoc comments must adhere to ASD-STE100 rules.
4. **Mandatory Commit Signatures:** All Git commits must be signed cryptographically (`git commit -S`).

---

## 5. Verification & Testing Discipline

> [!WARNING]
> Ceres runs multi-agent workloads. Unthrottled cargo commands cause memory exhaustion and process termination.

Always specify `-j 2` for all build and test invocations.

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -j 2 -- -D warnings
cargo test --workspace -j 2
```
