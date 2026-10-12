# blackwall-bridge (Blackwall Custody & Confinement Adapter)

Integration layer between the Escapement scheduler and Blackwall's isolated process execution and custody verification engine.

Part of [unsigned.gg](https://unsigned.gg).

| Attribute | Specification |
|---|---|
| Status | Functional, integration tests passing |
| Roadmap Phase | Phase 1 (Core Engine Foundation) |
| Layer | Layer 2 (Execution Bridge) |
| Depends On | `escapement-core`, `ep-domain` |
| Invariants | [`SPEC-0001 §4 Landlock Isolation`](../../specs/SPEC-0001-escapement-scheduler.md) |

## Overview

- **Crate Type:** `lib`
- **License:** `Apache-2.0 OR MIT OR CC0-1.0`

This crate maps an escapement [`Task`](../escapement-core/src/dispatch.rs) to a Blackwall run specification (prompt, provider, model, max turns, timeout) and maps Blackwall run records back to cryptographic [`CustodyRef`](../escapement-core/src/custody.rs) hashes.

## Core Modules

| Module | Description |
| :--- | :--- |
| **`execution`** | Manages plan execution pipelines and process lifecycle boundaries. |
| **`receive`** | Ingests completed run records from external workers into custody storage. |
| **`serve`** | Optional MCP tool dispatch interface (`POST /v1/mcp/tools/dispatch`). Enabled via `serve` feature flag. |
| **`settle`** | Verifies task settlement, final token expenditures, and BLAKE3 custody proofs. |

## Invariants & Design

1. **Subprocess Isolation:** The bridge executes the `blackwall` binary via `std::process::Command`, maintaining strong isolation without external network transport dependencies.
2. **Cryptographic Settlement:** Completed runs yield a 256-bit BLAKE3 hash representing execution history, verified before transitioning tasks to `Completed`.
3. **Controlled Error Propagation:** Non-zero exit codes from child sandboxes map to structured `BridgeError` variants without panics.

## Feature Flags

- **`serve`**: Activates HTTP serve bridge and MCP tool dispatch endpoints (`ServeBridge`, `ServeConfig`).

## Usage Example

```rust
use blackwall_bridge::{BridgeConfig, RunHandle};
use escapement_core::dispatch::Task;

let config = BridgeConfig::default();
// Bridge dispatches task into isolated blackwall sandbox
```

## Verification

Run all bridge integration tests under host throttling:

```bash
cargo test -j 2 -p blackwall-bridge
```

## License

Tri-licensed under `Apache-2.0 OR MIT OR CC0-1.0`.
