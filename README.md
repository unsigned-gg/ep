# ep — Escapement Scheduler & Deterministic Event Reactor

[![License: Apache-2.0 OR MIT](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue.svg)](https://github.com/unsigned-gg/.github)

> Metered, deterministic dispatch engine and event reactor for autonomous agent constellations.  
> Pure-Rust, zero-dependency core dispatch, capability matching, custody DAGs, and tick scheduling.  
> Dual-licensed under `Apache-2.0 OR MIT` and dedicated to the open testbed of mutual cooperation between man and machine.

## Philosophy & Foundations

- **Champion Inspiration:** Carl Lerche & Leslie Lamport
- **Zero Days Open Source Initiative (ZOSI):** Absolute machine sympathy, deterministic scheduling, and low-level mechanical efficiency.
- **Contract:** Zero zombies, strictly bounded memory queues, deterministic custody handoffs, and provable termination.

## Workspace Architecture

- **`crates/ep`**: Unified binary CLI and runtime interfaces.
- **`crates/ep-domain`**: Zero-I/O domain models, validation, and algebraic state machines.
- **`crates/escapement-core`**: Core deterministic dispatch engine, custody DAGs, task registry, capability matching, and telemetry.
- **`crates/escapement-serve`**: HTTP and gRPC orchestration server with Tempo/Jaeger OpenTelemetry tracing.
- **`crates/blackwall-bridge`**: Sandbox confinement adapter and isolated process execution bridge.

## Building & Verification

All cargo invocations on multi-agent hosts MUST specify `-j 2`:

```bash
cargo build -j 2
cargo test -j 2
```

## License

Dual-licensed under `Apache-2.0 OR MIT`.

## Principles

See the [unsigned-gg principles](https://github.com/unsigned-gg/.github) for the architectural doctrine and community standards.
