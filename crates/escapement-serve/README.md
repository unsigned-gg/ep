# escapement-serve (Orchestrator Daemon & HTTP API)

Runtime orchestrator daemon and network service for the Escapement engine.

Part of [unsigned.gg](https://unsigned.gg).

| Attribute | Specification |
|---|---|
| Status | Functional, 11 tests passing |
| Roadmap Phase | Phase 1 (Core Engine Foundation) |
| Layer | Layer 2 (Orchestration Server) |
| Depends On | `escapement-core`, `blackwall-bridge`, `ep-domain` |
| Invariants | [`SPEC-0001 §4 Single-Writer Dispatch`](../../specs/SPEC-0001-escapement-scheduler.md) |

## Overview

- **Binary Name:** `escapement-serve`
- **Crate Type:** `bin`, `lib`
- **License:** `Apache-2.0 OR MIT OR CC0-1.0`

The daemon integrates `escapement-core` scheduling primitives, `blackwall-bridge` execution adapters, and rate metering into a long-running service.

## Architecture & Single-Writer Model

1. **Single-Writer Core:** All queue mutations, state handoffs, and registry updates occur within a single coordinator task. This design eliminates lock contention and guarantees deterministic ordering.
2. **Asynchronous I/O Boundary:** Network endpoints accept incoming HTTP requests, serialize submissions into typed protocol envelopes, and submit them to the single-writer queue.
3. **OpenTelemetry Integration:** Emits distributed tracing spans and metrics over OTLP to Jaeger or Tempo collectors.

## HTTP Endpoints

| Method | Path | Description |
| :--- | :--- | :--- |
| **`GET`** | `/health` | Daemon liveness and readiness check. |
| **`POST`** | `/v1/tasks/admit` | Submit individual task for admission and capacity verification. |
| **`GET`** | `/v1/tasks/:id` | Query task execution status and custody chain. |
| **`POST`** | `/v1/plans/submit` | Submit Directed Acyclic Graph (DAG) plan for topological execution. |
| **`POST`** | `/v1/mcp/tools/dispatch` | MCP dispatch integration bridge for agent tool execution. |

## Configuration

The daemon loads configuration from `escapement.toml` or environment variables:

```toml
default_provider = "gateway"
default_model = "llm/glm-5.2"
default_max_turns = 20
max_tokens_per_run = 200000
max_retries = 3
tick_interval_ms = 100
otlp_endpoint = "http://localhost:4317"
```

## Running the Server

Start the daemon with host throttling:

```bash
cargo run -j 2 -p escapement-serve -- --config escapement.toml
```

## Verification

Run orchestrator unit and integration tests:

```bash
cargo test -j 2 -p escapement-serve
```

## License

Tri-licensed under `Apache-2.0 OR MIT OR CC0-1.0`.
