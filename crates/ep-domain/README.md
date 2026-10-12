# ep-domain (Zero-I/O Domain Models)

Pure zero-I/O domain models, algebraic state machines, and validation assertions for `ep`.

Part of [unsigned.gg](https://unsigned.gg).

| Attribute | Specification |
|---|---|
| Status | Functional, unit tests passing |
| Roadmap Phase | Phase 1 (Core Engine Foundation) |
| Layer | Layer 0 (Zero-I/O Domain Models) |
| Depends On | None (Pure Domain, zero workspace deps) |
| Invariants | [`SPEC-0001 §3 Zero-I/O Isolation`](../../specs/SPEC-0001-escapement-scheduler.md) |

## Architectural Invariants

1. **Zero-I/O Isolation:** This crate contains zero dependencies on `std::fs`, `std::net`, `std::process`, or asynchronous runtimes (`#![forbid(unsafe_code)]`).
2. **Pure Data Transformations:** All state models, transitions, and validation checks execute entirely in memory.
3. **Deterministic Serialization:** Models implement `Serialize` and `Deserialize` using `serde` for stable transport encoding.
4. **Controlled Language (ASD-STE100):** State models and transitions follow explicit, unambiguous definitions.

## Key Types

- [`EpState`](src/lib.rs): Core protocol versioning and state identity representation.

## Verification

Run domain tests under host throttling:

```bash
cargo test -j 2 -p ep-domain
```

## License

Tri-licensed under `Apache-2.0 OR MIT OR CC0-1.0`.
