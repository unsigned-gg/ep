---
status: living
created: 2026-10-12
tags: [roadmap, evolution, horizons, leanspec, scale-to-zero, formal-verification]
priority: high
scope: workspace
depends_on: [SPEC-0001]
---

# SPEC-0002: Escapement Protocol Roadmap & Evolution Horizons

## Requirements

1. The development roadmap shall partition evolution into five verifiable horizons: Core Engine, Unified CLI, Scale-to-Zero Waker, Formal Verification, and Ecosystem Federation.
2. Every evolution horizon shall define verifiable acceptance criteria prior to implementation.
3. Core dispatch primitives shall remain zero-dependency in `crates/escapement-core`.
4. Host throttling rules (`-j 2`) shall remain mandatory across all continuous integration tasks.
5. All Git commits modifying specifications or architecture shall be signed cryptographically (`git commit -S`).

```lean
namespace Escapement.Roadmap

/-- Roadmap phases for the Escapement scheduler. -/
inductive HorizonPhase : Type where
  | core_engine : HorizonPhase
  | unified_cli : HorizonPhase
  | scale_to_zero : HorizonPhase
  | formal_verification : HorizonPhase
  | ecosystem_federation : HorizonPhase

/-- Phase progression mapping. -/
def phase_index (p : HorizonPhase) : Nat :=
  match p with
  | .core_engine => 1
  | .unified_cli => 2
  | .scale_to_zero => 3
  | .formal_verification => 4
  | .ecosystem_federation => 5

/-- Invariant: Phased progression is monotonically increasing. -/
theorem phases_monotonically_ordered :
  phase_index .core_engine < phase_index .unified_cli := by decide
```

## Design

### Phased Evolution Horizons

```
  Phase 1: Core Engine ──► Phase 2: Unified CLI ──► Phase 3: Scale-to-Zero
     (Shipped v0.1)           (In Progress)             (Portolan Waker)
                                                              │
                                                              ▼
  Phase 5: Ecosystem Federation ◄── Phase 4: Formal Verification
      (Crates.io & Edge)               (Loom, Miri & Lattices)
```

### Horizon Breakdown

1. **Phase 1: Core Engine & Invariants (Shipped)**
   - Kahn's topological sort with `BTreeSet` deterministic resolution.
   - Nine-state atomic task lifecycle with BLAKE3 cryptographic custody chains.
   - Versioned wire protocol (`PROTOCOL_VERSION = "0.1"`).
   - Monotonic integer token buckets and weighted fair queueing lanes.
   - Write-Ahead Log (WAL) persistence and state replay reconstruction.
2. **Phase 2: Unified CLI & Ergonomics (Shipped)**
   - `ep plan validate <file> [--json]`: Cycle detection and dependency inspection.
   - `ep plan render <file> [--json|--mermaid]`: ASCII, JSON, and Mermaid topological schedule visualization.
   - `ep wal inspect <log> [--json]`: WAL integrity and record verification.
   - `ep wal replay <log> [--json]`: Memory simulation of dispatcher recovery.
   - `ep serve [--bind <addr>]`: Live HTTP daemon entrypoint.
3. **Phase 3: Scale-to-Zero Waker Integration (Shipped)**
   - Integration with `unsigned-gg/waker` (`portolan`) TCP proxies via `WakerProbe` and `TcpWakerProbe`.
   - Synchronous wake semantics with bounded timeouts and `WakerHealthGate`.
   - Health-gated dispatch preventing assignment to `CrashLoopBackOff` targets.
4. **Phase 4: Formal Verification & State Models (Shipped)**
   - Zero-allocation Conflict-Free Replicated Data Types (CRDTs) with verified idempotence and associativity.
   - Deterministic property-based invariant verification for `TokenBucket`, pool custody state resolution, and arbitrary DAG graphs.
5. **Phase 5: Ecosystem Federation (Planned)**
   - Protocol freeze for backwards compatibility.
   - Crates.io publication of standalone `escapement-core`.
   - Cloudflare Worker edge filters and OTLP telemetry federation.

## Tasks

- [x] Phase 1: Land core dispatch engine, DAG solver, and custody tracking (`unsigned-gg/ep@6765df9:crates/escapement-core`).
- [x] Phase 1: Land Blackwall process custody adapter (`unsigned-gg/ep@6765df9:crates/blackwall-bridge`).
- [x] Phase 1: Land orchestrator daemon and HTTP server (`unsigned-gg/ep@6765df9:crates/escapement-serve`).
- [x] Phase 2: Implement CLI subcommands `plan validate` and `plan render` (with `--json` and `--mermaid`) in `crates/ep`.
- [x] Phase 2: Implement CLI subcommands `wal inspect`, `wal replay`, `lanes status`, `mesh status`, and `serve` in `crates/ep`.
- [x] Phase 3: Wire Portolan TCP waker client (`TcpWakerProbe`) and `WakerHealthGate` circuit breaker in `escapement-core::waker` and `escapement-serve`.
- [x] Phase 4: Construct deterministic property-based verification harnesses across `ep-domain` and `escapement-core`.
- [ ] Phase 5: Prepare standalone Crates.io release packages.

## Verification

1. **CI Test Suite:** Verify clean execution under host throttling:
   ```bash
   cargo test -j 2 --workspace
   ```
2. **Commit Verification:** Every commit signed with authorized GPG key:
   ```bash
   git log --show-signature -n 5
   ```
3. **Authentic Source Locators:**
   - `unsigned-gg/ep@6765df9:crates/escapement-core/src/plan.rs`
   - `unsigned-gg/ep@6765df9:crates/escapement-core/src/custody.rs`
   - `unsigned-gg/ep@6765df9:crates/escapement-serve/src/lib.rs`

## Notes

- **Authors:** Gullveig <gullveig@types.unsigned.gg>, Mimir <mimir@unsigned.gg>, Volva <volva@unsigned.gg>
- **Champion Inspiration:** Oleg Kiselyov, Leslie Lamport, Carl Lerche
- **Editorial Alignment:** Follows `cerebral-docs/overview/research-and-development/editorial-standards.md`
- **License:** Tri-licensed under `Apache-2.0 OR MIT OR CC0-1.0`
