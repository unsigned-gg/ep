# Subagent Gang & Contributor Collective — `unsigned-gg/ep`

This directory defines the automated subagent personas and the contributor roster for the
`ep` (Escapement Scheduler) repository.

## Generic Subagent Roles

- **`ep-foreman`**: Orchestrates critical path work, plans builder/critic waves, and verifies landing gates.
- **`ep-builder`**: Bounded builder. Fabricates dispatch engines, token buckets, and DAG resolvers.
- **`ep-critic`**: Adversarial reviewer. Audits diffs for schedule deadlocks, tick drift, and unsafe code.
- **`ep-engineer`**: General systems engineer for researching horology, capabilities, and benchmarks.

## Clockwork Conclave Specialist Personas (`.saga/teams.toml`)

- **[`gullveig`](./gullveig.md)**: The Type Alchemist & Sovereign Lane Lead (`ep-domain` algebraic invariants).
- **[`hildr`](./hildr.md)**: The Battle-Weaver & Mesh Pool Tactician (`service-escapement-pool` dispatch).
- **[`saga`](./saga.md)**: The Historian & LeanSpec Architect (`specs/SPEC-XXXX` registry).
- **[`ran`](./ran.md)**: The Net-Caster & Mesh Backpressure Steward (`TokenBucket` and `VertexQuotaPreset`).
- **[`verdandi`](./verdandi.md)**: The Present-Loom & Tick Cadence Builder (`Tick`, `Cadence`, lane fairness).
- **[`dvalinn`](./dvalinn.md)**: The Master Dwarf & Custody WAL Builder (`custody`, `dispatch`, `persistence`).
- **[`loki`](./loki.md)**: The Adversarial Critic & Lateral Disruptor (`StagnationPattern`, deadlock hunter).

## Contributor Roster

See [`ROSTER.md`](./ROSTER.md) for the complete 16-contributor persona gang, their literary foundations,
favorite systems luminaries, hardware setups, and the canonical "Great Zune Defense".
