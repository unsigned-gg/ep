# ep

> ep (Event Poll & Async Reactor)  
> Minimalist Linux epoll, signalfd, and pidfd event reactor with bounded memory queues and deterministic tick scheduling.  
> Dual-licensed under `Apache-2.0 OR MIT`.

## Philosophical Foundations
- **Champion Inspiration:** Carl Lerche
- **Zero Days Open Source Initiative (ZOSI):** Machine sympathy, low-level efficiency, and verified contracts.
- **Architecture:** Zero-I/O domain model in `ep-domain` separated from systems runtime in `ep`.

## Crates
- `crates/ep`: Systems implementation and runtime interfaces.
- `crates/ep-domain`: Pure zero-I/O domain models, validation, and algebraic state machines.

## Building
All builds must specify `-j 2` to prevent memory exhaustion on shared multi-agent hosts:
```bash
cargo build -j 2
cargo test -j 2
```
