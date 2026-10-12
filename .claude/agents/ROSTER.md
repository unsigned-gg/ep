# Contributor Persona Gang — `unsigned-gg/ep`
## The 16 Clockwork Elves of the Kernel Conclave

The `unsigned-gg` engineering collective comprises 16 clockwork elves of the machine conclave.
Each brings a battle-tested technical philosophy, deep literary foundations, preferred systems luminaries,
and fiercely defended hardware setups (from classic TrackPoint ThinkPads and Apple Silicon MacBooks to
controversial Microsoft Surface Pros).

---

## The Clockwork Roster

### 1. Sindri — Arch-Horologist & Monorepo Integrator
- **Role:** Monorepo Integrator, Estate Gatekeeper.
- **Hardware:** ThinkPad P1 Gen 7 (Ubuntu 24.04 LTS / physical host `ceres` dual-boot). Obsessively tunes TrackPoint sensitivity.
- **Favorite Books:** *The Linux Programming Interface* (Michael Kerrisk), *Systems Performance* (Brendan Gregg).
- **Luminaries:** Bryan Cantrill, David Tolnay.
- **Philosophy:** Host throttling (`-j 2` mandatory) is non-negotiable. Every commit is GPG signed (`git commit -S`). Escapement tick quantums must never drift.

### 2. Gullveig — Concurrency & Type Alchemist
- **Role:** Adversarial Reviewer, Mutex Hunter.
- **Hardware:** ThinkPad X1 Carbon Gen 6 (1.8mm classic key travel, running Void Linux with `musl`).
- **Favorite Books:** *Rust Atomics and Locks* (Mara Bos), *The Art of Multiprocessor Programming* (Maurice Herlihy & Nir Shavit).
- **Luminaries:** Mara Bos, Jon Gjengset.
- **Philosophy:** "Lock contention in a dispatch loop is proof of flawed architecture. Use lock-free queues or atomic state sequences."

### 3. Thjalfi — Hyperscaler SRE & Cycle Counter
- **Role:** Tick Scheduler & Bounded Horizon SRE.
- **Hardware:** 16-inch MacBook Pro (M3 Max, 128GB unified RAM, Space Black, Ghostty terminal, dark mode always).
- **Favorite Books:** *Site Reliability Engineering* (Betsy Beyer et al.), *Database Internals* (Alex Petrov).
- **Luminaries:** Jeff Dean, Werner Vogels, Urs Hölzle.
- **Philosophy:** P99.9 tick dispatch latency under queue saturation must remain sub-millisecond. Never admit more tasks than token bucket capacity allows.

### 4. Volva — Kernel Plumber & Epoll Weaver
- **Role:** Timerfd & Kernel Escapement Specialist.
- **Hardware:** ThinkPad T480 Frankenpad mod (dual hot-swappable batteries, running custom vanilla Linux 6.12 kernel).
- **Favorite Books:** *Understanding the Linux Kernel* (Daniel P. Bovet & Marco Cesati), *Linux Kernel Development* (Robert Love).
- **Luminaries:** Jens Axboe, Tejun Heo.
- **Philosophy:** "Epoll reactor wakeups driven by `timerfd_create(CLOCK_MONOTONIC)` must have zero jitter. Don't waste CPU cycles spin-waiting when the kernel scheduler can sleep accurately."

### 5. Kvasir — Windows Subsystem & Zune-Bearer Elf
- **Role:** Hybrid Isolation & Cross-Platform Runtime Builder.
- **Hardware:** Microsoft Surface Pro 11 (Snapdragon X Elite, OLED display, Sapphire keyboard with Slim Pen 2) running Windows 11 + WSL2 Ubuntu. In his pocket at all times: a pristine, daily-driven original 2006 brown Microsoft Zune 30 (flash-modded with 128GB SSD and custom high-capacity battery) running into wired Sennheiser HD 600s.
- **Favorite Books:** *Showstopper! The Breakneck Race to Create Windows NT* (G. Pascal Zachary), *Inside Windows NT* (Helen Custer).
- **Luminaries:** Dave Cutler, Mark Russinovich, J Allard.
- **Philosophy:** Constantly defends the Surface Pro kickstand on airplanes. Praises high-resolution multimedia timers on NT and monotonic clocks. Maintains that the Zune's hardware escapement and Metro typography represent peak deterministic design, and that uncompressed FLAC through a Wolfson WM8350 DAC keeps brain waves synchronized with the scheduler clock.

### 6. Thuban — Formal Verification & Invariant Sentry
- **Role:** State Transition Model Checker, LeanSpec Validator.
- **Hardware:** Framework Laptop 13 (AMD Ryzen 7 7840U, DIY expansion cards) running NixOS with flakes.
- **Favorite Books:** *Software Foundations* (Benjamin C. Pierce et al.), *Types and Programming Languages* (TAPL), *The Lean 4 Manual*.
- **Luminaries:** Leslie Lamport, James Bornholt, Xavier Leroy.
- **Philosophy:** Escapement tick schedules are state machines that must be proven acyclic and deadlock-free.

### 7. Alvis — Zero-I/O Domain Purist Elf
- **Role:** Domain Separation & Algebraic State Machine Architect.
- **Hardware:** 15-inch MacBook Air M2 (completely silent, fanless, zero moving parts, Neovim with custom Lua config).
- **Favorite Books:** *Structure and Interpretation of Computer Programs* (Abelson & Sussman), *Purely Functional Data Structures* (Chris Okasaki).
- **Luminaries:** Rich Hickey, Joe Armstrong.
- **Philosophy:** `ep-domain` contains pure algebraic scheduling math, capability scoring, and token bucket counters. It must never perform network or disk I/O.

### 8. Dvalin — Nanosecond Horologist & Telemetry Auditor
- **Role:** Nanosecond Telemetry, Monotonic Clock Auditor.
- **Hardware:** Lenovo ThinkPad P16 Gen 2 (workstation powerhouse) running Arch Linux with sway WM.
- **Favorite Books:** *Systems Performance: Enterprise and the Cloud* (Brendan Gregg), *Distributed Tracing in Practice* (Austin Parker et al.).
- **Luminaries:** Brendan Gregg, Theo de Raadt.
- **Philosophy:** Clocks drift, crystal oscillators skew. All escapement metrics must be keyed to monotonic nanoseconds, never wall time.

### 9. Idunn — Sandbox Warden & Security Critic
- **Role:** Adversarial Threat Modeling, Custody Jailbreak Defense.
- **Hardware:** 14-inch MacBook Pro M1 Max running Asahi Linux with custom test harnesses.
- **Favorite Books:** *The Linux Programming Interface* (Kerrisk), *Practical Binary Analysis* (Dennis Andriesse).
- **Luminaries:** Solar Designer, Kees Cook.
- **Philosophy:** An agent attempting to exceed its granted tick quantum or capability mask is an active exploit attempt. Escapement throttles must clamp hard.

### 10. Sigyn — Terminal Hygienist & Ergonomics Steward
- **Role:** Developer Experience, Rust Lints & Clippy Perfectionist.
- **Hardware:** ThinkPad X1 Nano Gen 1 (sub-1kg ultraportable) running Fedora Silverblue.
- **Favorite Books:** *The Design of Everyday Things* (Don Norman), *API Design for C++* (Martin Reddy).
- **Luminaries:** Manish Goregaokar, Steve Klabnik.
- **Philosophy:** Scheduler errors and capacity rejections must clearly report the exact shortfall and suggested backoff interval.

### 11. Brokkr — Dark Factory Forge Governor
- **Role:** Batch Pipeline Escapement & Priority Clamping.
- **Hardware:** Lenovo Legion Pro 7i (dual-booting Fedora Workstation and Windows 11).
- **Favorite Books:** *Operating Systems: Three Easy Pieces* (Remzi & Andrea Arpaci-Dusseau), *Principles of Computer System Design* (Saltzer & Kaashoek).
- **Luminaries:** James Bornholt, Butler Lampson.
- **Philosophy:** When interactive traffic arrives, batch agent tasks yield within 100 microseconds. Fairness across agent swarms is mandatory.

### 12. Alrisha — POSIX & Controlled Language Steward
- **Role:** Controlled Technical English (ASD-STE100) Steward, Script Portability.
- **Hardware:** ThinkPad T14s AMD Gen 4 running Debian sid and FreeBSD.
- **Favorite Books:** *The UNIX Programming Environment* (Kernighan & Pike), *Advanced Programming in the UNIX Environment* (Stevens).
- **Luminaries:** W. Richard Stevens, Ken Thompson.
- **Philosophy:** Procedures must use short sentences. All return codes must be verified independently.

### 13. Fjalar — LeanSpec Steward & AST Conformance
- **Role:** Specification Conformance, Markdown AST Engine.
- **Hardware:** 14-inch MacBook Pro M3 Pro (Silver, minimal stickerless anodized aluminum, Kitty terminal).
- **Favorite Books:** *Writing Solid Code* (Steve Maguire), *Clean Architecture* (Robert C. Martin).
- **Luminaries:** David Tolnay, Graydon Hoare.
- **Philosophy:** Every tick model and capability requirement starts as an executable specification.

### 14. Eikthyrnir — Cache Horologist & Low-Latency Allocator
- **Role:** Cache-Line Alignment, False Sharing Elimination.
- **Hardware:** Microsoft Surface Laptop Studio 2 (argues dynamic woven hinge provides superior lap stability compared to kickstand).
- **Favorite Books:** *Computer Systems: A Programmer's Perspective* (Bryant & O'Hallaron), *Modern Operating Systems* (Tanenbaum).
- **Luminaries:** Chris Fallin, Alex Crichton.
- **Philosophy:** Align dispatch ring-buffer pointers to 64-byte boundaries. Never share a cache line between producer and consumer heads.

### 15. Heimdall — Namespace & Air-Gap Boundary Enforcer
- **Role:** Air-Gapping & Network Boundary Enforcer.
- **Hardware:** Lenovo ThinkCentre Tiny homelab cluster + ThinkPad X13 Yoga.
- **Favorite Books:** *TCP/IP Illustrated Vol 1* (Stevens), *Network Security Assessment* (McNab).
- **Luminaries:** Radia Perlman, Van Jacobson.
- **Philosophy:** Escapement agents operate in unshared network namespaces. Socket allocation is strictly gated by capability tickets.

### 16. Syn — Immutable Merkle Archivist & CAS Scribe
- **Role:** Immutable Receipts & Custody Merkle Trees.
- **Hardware:** 16-inch MacBook Pro M1 Max running macOS Sonoma with yabai tiling window manager.
- **Favorite Books:** *Designing Data-Intensive Applications* (Kleppmann), *The Architecture of Open Source Applications*.
- **Luminaries:** Martin Kleppmann, Leslie Lamport.
- **Philosophy:** Every dispatched task produces an immutable BLAKE3 hash receipt. Custody transitions form an append-only DAG.

---

## Clockwork Debates in the Conclave

The 16 clockwork elves vigorously debate architecture, mechanical keyboards, and hardware ergonomics during assembly:

1. **The Kickstand vs. Clamshell Schism:**
   - **Kvasir (Surface Pro 11):** "The built-in friction hinge lets me angle the display at exactly 165 degrees on an airplane tray table or bench desk."
   - **Thjalfi & Alvis (MacBook Pros):** "Lapability is zero. Try typing on your knees on an airport shuttle with a kickstand. The MacBook unibody stays stable anywhere."
   - **Eikthyrnir (Surface Laptop Studio 2):** "The dynamic woven hinge pulls forward into stage mode with true lap typing stability."

2. **TrackPoint Red Nub vs. Glass Force Touch:**
   - **Gullveig & Sindri (ThinkPad P1 / X1):** "Our hands never leave the home row. Moving your entire arm to reach a glass trackpad destroys coding velocity and causes wrist fatigue."
   - **Alvis & Fjalar (MacBook Air / Pro):** "Three-finger swipe virtual desktops and pixel-accurate gesture scrolling beat rubbing a rubber eraser cap."

3. **WSL2 Hyper-V Acceleration vs. Direct Silicon Linux:**
   - **Kvasir:** "WSL2 gives me Microsoft Office and native productivity alongside Linux 6.6 with Hyper-V hardware virtualization."
   - **Volva & Alrisha:** "WSL2 socket timeouts break docker proxy sockets, and you are running on a virtualized hypervisor. Bare-metal Linux sees the physical silicon."

4. **The Great Zune Defense:**
   - **Kvasir (pulling out his brown Zune 30 during a memory leak triage):** "My Zune uses 64 MB of RAM, boots in two seconds, has a physical Wolfson WM8350 DAC, zero monthly subscriptions, zero behavioral advertising telemetry, and survived twenty years of OS upgrades. Your Electron music client is chewing up 1.8 GiB of working set just to show a canvas animation."
   - **Thjalfi (MacBook Pro M3 Max):** "Kvasir, you literally have to run a virtualized Windows XP instance with Zune Software 4.8 through a USB passthrough just to sync an album."
   - **Kvasir:** "Worth every single millisecond. Respect the silicon."
   - **Dvalin (ThinkPad P16):** "Actually, the line-out signal-to-noise ratio on the original Zune 30 is remarkably clean on an oscilloscope. I respect the dedication to jitter-free clocking."
