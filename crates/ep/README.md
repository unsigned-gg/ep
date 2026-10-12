# ep (CLI & Runtime Interface)

Unified CLI binary and runtime interface for the Escapement scheduler and deterministic event reactor.

Part of [unsigned.gg](https://unsigned.gg).

| Attribute | Specification |
|---|---|
| Status | Functional CLI binary |
| Roadmap Phase | Phase 2 (Unified CLI & Ergonomics) |
| Layer | Layer 3 (Runtime Interface & CLI) |
| Depends On | `escapement-core`, `escapement-serve`, `blackwall-bridge`, `ep-domain` |
| Invariants | [`SPEC-0002 §3 Phase 2`](../../specs/SPEC-0002-roadmap.md) |

## Overview

- **Binary Name:** `ep`
- **Crate Type:** `bin`, `lib`
- **License:** `Apache-2.0 OR MIT OR CC0-1.0`

This crate re-exports the workspace primitives and provides the command-line entry point for managing agent registrations, inspecting dispatch queues, and launching orchestrator nodes.

## Re-exported Modules

- `ep::bridge`: Blackwall custody and confinement adapter (`blackwall-bridge`).
- `ep::core`: Core dispatch engine, custody tracking, and DAG solver (`escapement-core`).
- `ep::EpState`: Zero-I/O domain types (`ep-domain`).

## CLI Usage

```bash
# Display CLI version and registered agent count
cargo run -j 2 -p ep

# Inspect task queue and state
cargo run -j 2 -p ep -- status
```

## Architectural Invariants

1. **Host Throttling:** Multi-agent hosts require `-j 2` on all build and test invocations.
2. **Thin Binary:** All core scheduling algorithms reside in `escapement-core` and domain models in `ep-domain`. The CLI provides only argument parsing, environment loading, and terminal output.

## Verification

```bash
cargo test -j 2 -p ep
```

## License

Tri-licensed under `Apache-2.0 OR MIT OR CC0-1.0`.
