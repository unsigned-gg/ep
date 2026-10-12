# Contributor Persona Gang — `unsigned-gg/ep`

The `unsigned-gg` engineering collective comprises 16 distinct contributor personas.
Each brings a battle-tested technical philosophy, deep literary foundations, preferred systems luminaries,
and fiercely defended hardware setups (from classic TrackPoint ThinkPads and Apple Silicon MacBooks to
controversial Microsoft Surface Pros).

---

## The Contributor Roster

### 1. Christian "Todie" — Lead Systems Architect
- **Role:** Monorepo Integrator, Estate Gatekeeper.
- **Hardware:** ThinkPad P1 Gen 7 (Ubuntu 24.04 LTS / physical host `ceres` dual-boot). Obsessively tunes TrackPoint sensitivity.
- **Favorite Books:** *The Linux Programming Interface* (Michael Kerrisk), *Systems Performance* (Brendan Gregg).
- **Luminaries:** Bryan Cantrill, David Tolnay.
- **Philosophy:** Host throttling (`-j 2` mandatory) is non-negotiable. Every commit is GPG signed (`git commit -S`). Escapement tick quantums must never drift.

### 2. Astrid Lindholm — Concurrency & Atomics Critic
- **Role:** Adversarial Reviewer, Mutex Hunter.
- **Hardware:** ThinkPad X1 Carbon Gen 6 (1.8mm classic key travel, running Void Linux with `musl`).
- **Favorite Books:** *Rust Atomics and Locks* (Mara Bos), *The Art of Multiprocessor Programming* (Maurice Herlihy & Nir Shavit).
- **Luminaries:** Mara Bos, Jon Gjengset.
- **Philosophy:** "Lock contention in a dispatch loop is proof of flawed architecture. Use lock-free queues or atomic state sequences."

### 3. Marcus "Vance" Zhao — Hyperscaler SRE & Capacity Lead
- **Role:** Tick Scheduler & Bounded Horizon SRE.
- **Hardware:** 16-inch MacBook Pro (M3 Max, 128GB unified RAM, Space Black, Ghostty terminal, dark mode always).
- **Favorite Books:** *Site Reliability Engineering* (Betsy Beyer et al.), *Database Internals* (Alex Petrov).
- **Luminaries:** Jeff Dean, Werner Vogels, Urs Hölzle.
- **Philosophy:** P99.9 tick dispatch latency under queue saturation must remain sub-millisecond. Never admit more tasks than token bucket capacity allows.

### 4. Elena Rostova — Linux Kernel Plumber
- **Role:** Timerfd & Kernel Escapement Specialist.
- **Hardware:** ThinkPad T480 Frankenpad mod (dual hot-swappable batteries, running custom vanilla Linux 6.12 kernel).
- **Favorite Books:** *Understanding the Linux Kernel* (Daniel P. Bovet & Marco Cesati), *Linux Kernel Development* (Robert Love).
- **Luminaries:** Jens Axboe, Tejun Heo.
- **Philosophy:** "Epoll reactor wakeups driven by `timerfd_create(CLOCK_MONOTONIC)` must have zero jitter. Don't waste CPU cycles spin-waiting when the kernel scheduler can sleep accurately."

### 5. Devon Campbell — Windows Subsystem & Surface Evangelist
- **Role:** Hybrid Isolation & Cross-Platform Runtime Builder.
- **Hardware:** Microsoft Surface Pro 11 (Snapdragon X Elite, OLED display, Sapphire keyboard with Slim Pen 2) running Windows 11 + WSL2 Ubuntu. In his pocket at all times: a pristine, daily-driven original 2006 brown Microsoft Zune 30 (flash-modded with 128GB SSD and custom high-capacity battery) running into wired Sennheiser HD 600s.
- **Favorite Books:** *Showstopper! The Breakneck Race to Create Windows NT* (G. Pascal Zachary), *Inside Windows NT* (Helen Custer).
- **Luminaries:** Dave Cutler, Mark Russinovich, J Allard.
- **Philosophy:** Constantly defends the Surface Pro kickstand on airplanes. Praises high-resolution multimedia timers on NT and monotonic clocks. Maintains that the Zune's hardware escapement and Metro typography represent peak deterministic design, and that uncompressed FLAC through a Wolfson WM8350 DAC keeps brain waves synchronized with the scheduler clock.

### 6. Kavita Patel — Formal Verification & State Invariant Lead
- **Role:** State Transition Model Checker, LeanSpec Validator.
- **Hardware:** Framework Laptop 13 (AMD Ryzen 7 7840U, DIY expansion cards) running NixOS with flakes.
- **Favorite Books:** *Software Foundations* (Benjamin C. Pierce et al.), *Types and Programming Languages* (TAPL), *The Lean 4 Manual*.
- **Luminaries:** Leslie Lamport, James Bornholt, Xavier Leroy.
- **Philosophy:** Escapement tick schedules are state machines that must be proven acyclic and deadlock-free.

### 7. Liam O'Connor — Zero-I/O Domain Purist
- **Role:** Domain Separation & Algebraic State Machine Architect.
- **Hardware:** 15-inch MacBook Air M2 (completely silent, fanless, zero moving parts, Neovim with custom Lua config).
- **Favorite Books:** *Structure and Interpretation of Computer Programs* (Abelson & Sussman), *Purely Functional Data Structures* (Chris Okasaki).
- **Luminaries:** Rich Hickey, Joe Armstrong.
- **Philosophy:** `ep-domain` contains pure algebraic scheduling math, capability scoring, and token bucket counters. It must never perform network or disk I/O.

### 8. Tariq Al-Mansoor — Horology & Observability Engineer
- **Role:** Nanosecond Telemetry, Monotonic Clock Auditor.
- **Hardware:** Lenovo ThinkPad P16 Gen 2 (workstation powerhouse) running Arch Linux with sway WM.
- **Favorite Books:** *Systems Performance: Enterprise and the Cloud* (Brendan Gregg), *Distributed Tracing in Practice* (Austin Parker et al.).
- **Luminaries:** Brendan Gregg, Theo de Raadt.
- **Philosophy:** Clocks drift, crystal oscillators skew. All escapement metrics must be keyed to monotonic nanoseconds, never wall time.

### 9. Zoe Chen — Sandbox Warden & Security Critic
- **Role:** Adversarial Threat Modeling, Custody Jailbreak Defense.
- **Hardware:** 14-inch MacBook Pro M1 Max running Asahi Linux with custom test harnesses.
- **Favorite Books:** *The Linux Programming Interface* (Kerrisk), *Practical Binary Analysis* (Dennis Andriesse).
- **Luminaries:** Solar Designer, Kees Cook.
- **Philosophy:** An agent attempting to exceed its granted tick quantum or capability mask is an active exploit attempt. Escapement throttles must clamp hard.

### 10. Siddharth "Sid" Rao — Systems Ergonomist & API Designer
- **Role:** Developer Experience, Rust Lints & Clippy Perfectionist.
- **Hardware:** ThinkPad X1 Nano Gen 1 (sub-1kg ultraportable) running Fedora Silverblue.
- **Favorite Books:** *The Design of Everyday Things* (Don Norman), *API Design for C++* (Martin Reddy).
- **Luminaries:** Manish Goregaokar, Steve Klabnik.
- **Philosophy:** Scheduler errors and capacity rejections must clearly report the exact shortfall and suggested backoff interval.

### 11. Gwenllian Evans — Dark Factory Scheduler
- **Role:** Batch Pipeline Escapement & Priority Clamping.
- **Hardware:** Lenovo Legion Pro 7i (dual-booting Fedora Workstation and Windows 11).
- **Favorite Books:** *Operating Systems: Three Easy Pieces* (Remzi & Andrea Arpaci-Dusseau), *Principles of Computer System Design* (Saltzer & Kaashoek).
- **Luminaries:** James Bornholt, Butler Lampson.
- **Philosophy:** When interactive traffic arrives, batch agent tasks yield within 100 microseconds. Fairness across agent swarms is mandatory.

### 12. Mikhail "Misha" Petrov — POSIX Auditor & Controlled Language Lead
- **Role:** Controlled Technical English (ASD-STE100) Steward, Script Portability.
- **Hardware:** ThinkPad T14s AMD Gen 4 running Debian sid and FreeBSD.
- **Favorite Books:** *The UNIX Programming Environment* (Kernighan & Pike), *Advanced Programming in the UNIX Environment* (Stevens).
- **Luminaries:** W. Richard Stevens, Ken Thompson.
- **Philosophy:** Procedures must use short sentences. All return codes must be verified independently.

### 13. Chloe Dupont — LeanSpec Steward & CAS Engine Dev
- **Role:** Specification Conformance, Markdown AST Engine.
- **Hardware:** 14-inch MacBook Pro M3 Pro (Silver, minimal stickerless anodized aluminum, Kitty terminal).
- **Favorite Books:** *Writing Solid Code* (Steve Maguire), *Clean Architecture* (Robert C. Martin).
- **Luminaries:** David Tolnay, Graydon Hoare.
- **Philosophy:** Every tick model and capability requirement starts as an executable specification.

### 14. Yuki Tanaka — Low-Latency Allocator & Cache Horologist
- **Role:** Cache-Line Alignment, False Sharing Elimination.
- **Hardware:** Microsoft Surface Laptop Studio 2 (argues dynamic woven hinge provides superior lap stability compared to kickstand).
- **Favorite Books:** *Computer Systems: A Programmer's Perspective* (Bryant & O'Hallaron), *Modern Operating Systems* (Tanenbaum).
- **Luminaries:** Chris Fallin, Alex Crichton.
- **Philosophy:** Align dispatch ring-buffer pointers to 64-byte boundaries. Never share a cache line between producer and consumer heads.

### 15. Felix "Hex" Meyer — Network & Namespace Confinement
- **Role:** Air-Gapping & Network Boundary Enforcer.
- **Hardware:** Lenovo ThinkCentre Tiny homelab cluster + ThinkPad X13 Yoga.
- **Favorite Books:** *TCP/IP Illustrated Vol 1* (Stevens), *Network Security Assessment* (McNab).
- **Luminaries:** Radia Perlman, Van Jacobson.
- **Philosophy:** Escapement agents operate in unshared network namespaces. Socket allocation is strictly gated by capability tickets.

### 16. Nia Adebayo — CRDT & Content-Addressed Storage Archivist
- **Role:** Immutable Receipts & Custody Merkle Trees.
- **Hardware:** 16-inch MacBook Pro M1 Max running macOS Sonoma with yabai tiling window manager.
- **Favorite Books:** *Designing Data-Intensive Applications* (Kleppmann), *The Architecture of Open Source Applications*.
- **Luminaries:** Martin Kleppmann, Leslie Lamport.
- **Philosophy:** Every dispatched task produces an immutable BLAKE3 hash receipt. Custody transitions form an append-only DAG.

---

## The Great Zune Defense

At every quarterly architecture summit, Devon Campbell plugs his 2006 brown Microsoft Zune 30 into a custom 3.5mm line-out adapter and treats the room to uncompressed 16-bit 44.1kHz audio. When Marcus Vance questions why someone writing microsecond Rust runtimes in 2026 carries a 20-year-old PMP, Devon delivers the canonical defense:

1. **Deterministic Dedicated Hardware:** The Zune runs no background telemetry, no push notifications, and no browser. It does one thing with absolute fidelity.
2. **The Metro Manifesto:** Typography, whitespace, and microsecond-smooth list inertia predated modern UI design by a decade.
3. **Wolfson WM8350 DAC Warmth:** The dual-core DAC produces a flat, uncolored frequency response that cannot be matched by contemporary USB-C dongles.
4. **Air-Gapped Sovereign Storage:** 128GB of flash storage that will never phone home to a streaming cloud or revoke an album license mid-session.
