# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Comprehensive crate-level documentation and README files across all workspace crates (`crates/ep`, `crates/ep-domain`, `crates/escapement-core`, `crates/escapement-serve`, `crates/blackwall-bridge`).
- LeanSpec frontmatter and canonical section alignment (`Requirements`, `Design`, `Tasks`, `Verification`, `Notes`) for `SPEC-0001` and `SPEC-0002`.
- Tri-license documentation block (`Apache-2.0 OR MIT OR CC0-1.0`) across root and crate documentation.
- Specification registry and authentic Git commit locators in `specs/README.md`.
- Keep-a-Changelog scaffolding and tracking file `CHANGELOG.md`.

### Changed
- Restructured root `README.md` to include Crate Responsibility Matrix, Zero-I/O Domain Isolation rules, host throttling invariants, and concrete CLI usage examples.
- Updated `ROADMAP.md` to reference crate READMEs and formal LeanSpec specifications.

### Deprecated

### Removed

### Fixed

### Security

## [0.1.0] - 2026-10-12

### Added
- Metered, deterministic dispatch engine and event reactor workspace in pure Rust.
- Core dispatch, capability matching, custody DAGs, priority lanes, and token bucket rate limiting in `escapement-core`.
- Zero-I/O domain types and algebraic state machines in `ep-domain`.
- Single-writer orchestrator daemon and HTTP service with OpenTelemetry export in `escapement-serve`.
- Blackwall custody confinement adapter and execution bridge in `blackwall-bridge`.
- Unified binary CLI entrypoint and registry inspection in `ep`.
- Formal specifications `SPEC-0001` (Escapement Scheduler) and `SPEC-0002` (Roadmap & Evolution Horizons).
