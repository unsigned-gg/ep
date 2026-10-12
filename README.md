# ep — Escapement Scheduler & Deterministic Event Reactor

[![License: Apache-2.0 OR MIT OR CC0-1.0](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT%20OR%20CC0--1.0-blue.svg)](https://github.com/unsigned-gg/.github)

> Metered, deterministic dispatch engine and event reactor for autonomous agent constellations.  
> Pure-Rust workspace with zero-dependency core dispatch, capability matching, custody DAGs, and tick scheduling.  
> Tri-licensed under `Apache-2.0 OR MIT OR CC0-1.0` and dedicated to the open testbed of mutual cooperation between man and machine.

## Philosophy & Foundations

- **Champion Inspiration:** Carl Lerche, Leslie Lamport, and Mara Bos.
- **Zero Days Open Source Initiative (ZOSI):** Absolute machine sympathy, deterministic scheduling, and low-level mechanical efficiency.
- **Contract:** Zero zombies, strictly bounded memory queues, deterministic custody handoffs, and provable termination.

## Architectural Invariants & Machine Sympathy

1. **Substantive Systems Implementation:** Fully functional deterministic scheduler and event reactor with zero vanity stubs.
2. **Zero-I/O Domain Isolation:** The `ep-domain` crate maintains zero dependencies on `std::fs`, `std::net`, `std::process`, or asynchronous runtimes. All domain models and state transitions execute entirely in memory.
3. **Controlled Language (ASD-STE100):** Documentation and specifications follow Simplified Technical English standards for precision and clarity.
4. **Host Throttling (`-j 2` Mandatory):** All cargo invocations on multi-agent hosts MUST specify `-j 2` to prevent CPU starvation and memory exhaustion.
5. **Deterministic Topological Scheduling:** Task execution DAGs validate cycle freedom and resolve deterministic execution order sorted by `(priority, submit_time, node_id)`.
6. **Zero Zombie Custody:** Admitted tasks transition through atomic compare-and-swap handoffs. No task remains unassigned without an active timeout watch.
7. **Monotonic Integer Metering:** Capabilities and rate limits use discrete integer token buckets refilled monotonically on tick boundaries without floating-point arithmetic.

## Crate Responsibility Matrix

| Crate | Path | Responsibility |
| :--- | :--- | :--- |
| **`ep`** | [`crates/ep`](crates/ep) | Unified CLI binary and runtime interface. Dispatches commands, inspects registries, and manages local daemon instances. |
| **`ep-domain`** | [`crates/ep-domain`](crates/ep-domain) | Pure zero-I/O domain types, algebraic state machines, and validation logic. |
| **`escapement-core`** | [`crates/escapement-core`](crates/escapement-core) | Core deterministic dispatch engine, custody DAGs, capability matching, token buckets, priority lanes, WAL persistence, and telemetry. |
| **`escapement-serve`** | [`crates/escapement-serve`](crates/escapement-serve) | Single-writer orchestrator daemon, HTTP and gRPC orchestration server with OpenTelemetry tracing export. |
| **`blackwall-bridge`** | [`crates/blackwall-bridge`](crates/blackwall-bridge) | Sandbox confinement adapter and isolated process execution bridge for Blackwall run custody. |

## CLI Usage

### Inspect Local Registry
```bash
# Print escapement banner and query active agents in the registry
cargo run -j 2 -p ep
```

### Run Orchestrator Daemon
```bash
# Start the single-writer HTTP orchestrator daemon on port 8080
cargo run -j 2 -p escapement-serve -- --config escapement.toml
```

### Submit Dispatched Run to Blackwall Bridge
```bash
# Verify isolated process custody through blackwall bridge
cargo test -j 2 -p blackwall-bridge
```

## Building & Verification

All cargo invocations on host `ceres` MUST specify `-j 2`:

```bash
# Compile entire workspace under host throttling
cargo build -j 2 --workspace

# Run full test suite (162 unit and integration tests)
cargo test -j 2 --workspace

# Generate local workspace documentation
cargo doc -j 2 --workspace --no-deps
```

## Formal Specifications & Roadmap

- **Roadmap Overview**: See [`ROADMAP.md`](ROADMAP.md) for evolution horizons and milestones.
- **Formal Specifications**: See [`specs/`](specs/) for LeanSpec contracts and operational invariants:
  - [`SPEC-0001: Escapement Scheduler & Deterministic Event Reactor`](specs/SPEC-0001-escapement-scheduler.md) (`unsigned-gg/ep@6765df9:specs/SPEC-0001-escapement-scheduler.md`)
  - [`SPEC-0002: Escapement Protocol Roadmap & Evolution Horizons`](specs/SPEC-0002-roadmap.md) (`unsigned-gg/ep@6765df9:specs/SPEC-0002-roadmap.md`)

## License

Tri-licensed under `Apache-2.0 OR MIT OR CC0-1.0`.

## Principles

See the [unsigned-gg principles](https://github.com/unsigned-gg/.github) for architectural doctrine and community standards.
