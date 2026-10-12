# Architecture Specifications (LeanSpec Registry)

> Specifications precede implementation. If a system's invariants cannot be
> articulated in plain technical English, compiling it is merely executing an
> unvetted guess.

This directory is the LeanSpec registry of record for `unsigned-gg/ep`. Every architectural decision, boundary rule, and operational invariant is specified as an individual, versioned contract adhering to ASD-STE100 and LeanSpec standards.

## Specification Index

| Spec | Title | Status | Priority | Scope | Tags | Primary Source Locator |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| [**SPEC-0001**](SPEC-0001-escapement-scheduler.md) | Escapement Scheduler & Deterministic Event Reactor | `complete` | `critical` | `escapement-core` | `scheduler`, `custody`, `metering` | `unsigned-gg/ep@6765df9:specs/SPEC-0001-escapement-scheduler.md` |
| [**SPEC-0002**](SPEC-0002-roadmap.md) | Escapement Protocol Roadmap & Evolution Horizons | `living` | `high` | `workspace` | `roadmap`, `scale-to-zero`, `formal-verification` | `unsigned-gg/ep@6765df9:specs/SPEC-0002-roadmap.md` |

## LeanSpec Specification Format

Every specification document in this registry follows the canonical structure:

1. **YAML Frontmatter:**
   - `status`: `planned`, `draft`, `reviewed`, `living`, `complete`, or `superseded`.
   - `created`: Date of creation (`YYYY-MM-DD`).
   - `tags`: Bounded categorization terms.
   - `priority`: `critical`, `high`, `medium`, or `low`.
   - `scope`: Crate or architectural component targeted.
   - `depends_on`: Upstream specification prerequisites.
2. **Canonical Sections (Exact Sequence):**
   - `## Requirements`: Plain-text invariant statements with embedded Lean 4 proof blocks.
   - `## Design`: Architectural diagrams, algebraic state machine tables, and structural flows.
   - `## Tasks`: Checkable implementation and tracking steps.
   - `## Verification`: Reproducible commands, test coverage criteria, and authentic Git source locators.
   - `## Notes`: Authors, licensing, champion inspirations, and cross-references.

## Adding a Specification

1. Select the next sequential identifier (`SPEC-0003`, `SPEC-0004`).
2. Write specifications using Simplified Technical English (ASD-STE100).
3. Embed formal verification statements or Lean 4 models for mathematical invariants.
4. Verify all build and test steps under host throttling (`-j 2`).
5. Register the new specification in the registry table above.
6. Reference research and editorial standards in `cerebral-docs/overview/research-and-development/editorial-standards.md`.
