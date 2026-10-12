# Architecture Specifications (LeanSpec)

> Specifications precede implementation. If a system's invariants cannot be
> articulated in plain technical English, compiling it is merely executing an
> unvetted guess.

This directory is the LeanSpec registry of record for `ep`. Every architectural decision,
boundary rule, and verification gate is specified as an individual, versioned contract.

| Spec | Title | Status | Priority |
|---|---|---|---|
| [SPEC-0001](SPEC-0001-escapement-scheduler.md) | Escapement Scheduler & Deterministic Event Reactor | complete | critical |
| [SPEC-0002](SPEC-0002-roadmap.md) | Escapement Protocol Roadmap & Evolution Horizons | complete | high |

## Specification Lifecycle

- `planned`: Under architectural evaluation.
- `in-progress`: Implementation or formal verification active.
- `complete`: Implemented with verified test suites and operational contracts.
- `archived`: Deprecated or superseded.

## How to Add a Spec

1. Select the next sequential spec identifier (e.g., `SPEC-0003`).
2. Write specifications using ASD-STE100 controlled language precision.
3. Formulate mathematical foundations and operational invariants explicitly.
4. Provide verifiable, reproducible test commands under `-j 2` host throttling.
5. Register the new specification in the table above.
