# ep — Roadmap

> Metered, deterministic dispatch engine and event reactor for autonomous agent constellations.  
> Repo: `unsigned-gg/ep` · Specification: [`specs/SPEC-0002-roadmap.md`](specs/SPEC-0002-roadmap.md)

---

## Current State (v0.1.0 Foundation)

- **Foundation Core Shipped**: 162 unit tests passing, clippy clean (`-D warnings`), 5 crates documented.
- **[`escapement-core`](crates/escapement-core/README.md)**: Zero-dependency pure-Rust engine:
  - Deterministic DAG topological resolver (`plan.rs`) with Kahn's algorithm and `BTreeSet` tie-breaking.
  - Nine-state atomic task lifecycle state machine (`dispatch.rs`).
  - BLAKE3 cryptographic custody provenance and chain propagation (`custody.rs`).
  - Strongly typed wire protocol (`protocol.rs`) with version negotiation (`PROTOCOL_VERSION = "0.1"`).
  - Leaky token-bucket metering and weighted fair queueing scheduler (`metering.rs`, `lanes.rs`).
  - Append-only Write-Ahead Log (WAL) persistence and crash recovery (`persistence.rs`).
  - Adaptive backpressure, tenant budgets, and scale-to-zero waker hooks (`backpressure.rs`, `budget.rs`, `waker.rs`).
- **[`blackwall-bridge`](crates/blackwall-bridge/README.md)**: Landlock sandbox execution adapter.
- **[`ep-domain`](crates/ep-domain/README.md)**: Zero-I/O domain types and algebraic state models.
- **[`escapement-serve`](crates/escapement-serve/README.md)**: Single-writer daemon prototype with HTTP endpoints (`/health`, `/v1/tasks/admit`, `/v1/plans/submit`) and OTLP tracing.
- **[`ep`](crates/ep/README.md)**: Unified CLI runtime interface.

---

## Development Horizons

See [`specs/SPEC-0002-roadmap.md`](specs/SPEC-0002-roadmap.md) for full formal specifications and operational invariants.

### Phase 2: Unified CLI & Ergonomics (In Progress)
- Interactive CLI subcommands in `crates/ep`: `ep plan validate`, `ep plan render`, `ep wal inspect`, `ep wal replay`, `ep lanes status`.
- Dual output modes: ASD-STE100 calibrated plain text and structured JSON.

### Phase 3: Scale-to-Zero & Waker Integration (Planned)
- Direct integration with `unsigned-gg/waker` (`portolan`) TCP wake proxy via `wake_and_wait(target)`.
- CrashLoopBackOff detection and health-gated task dispatch.
- Multi-cluster lane distribution across Cygnus and Orion topologies.

### Phase 4: Formal Verification & Algebraic Lattices (Planned)
- Conflict-Free Replicated Data Types (CRDTs) in `merge.rs` for distributed task merging.
- Type-state lifecycle constraints enforcing compile-time custody guarantees.
- Stateright / Loom model checking for lock-free concurrency and Proptest invariant testing.
- Undefined behavior validation under Miri.

### Phase 5: Crates.io Publication & Ecosystem Federation (Planned)
- Protocol freeze at `"1.0"`.
- Standalone zero-dependency crates.io publication for `escapement-core`.
- Cloudflare Worker edge gateway (`edge/worker`) admission validation.
- Constellation-wide OpenTelemetry trace and metric consolidation.

---

## Integration with the Estate

- **`soma` → `ep`**: Reflex routes metered agent task graphs directly into escapement priority lanes.
- **`ep` → `blackwall`**: Escapement spawns confined agent executions through `blackwall-bridge` with Landlock isolation.
- **`ep` → `waker`**: Scale-to-zero microVM workers awaken on demand through TCP wake proxies before task assignment.
- **`ep` → `reverie`**: Execution telemetry and content-addressed custody chains propagate to reverie memory records.
