# ADR 0007: Core Runtime Implementation in Rust

- **Status:** Accepted
- **Date:** 2026-10-15
- **Decision owners:** Engineering Lead, System Architect, Security Lead

## Context

Phase 1 requires implementing the core foundational components of UMR:
1. An unprivileged, sandboxed package intake worker handling untrusted archive formats (ZIP/APK/IPA) and complex binary file parsers (`AndroidManifest.xml`, `Info.plist`, DEX headers, Mach-O headers).
2. A lifecycle orchestrator managing concurrent state machines, timeouts, process supervisors, and structured event streaming.
3. Stable backend contracts defining interfaces for heterogeneous execution runners.
4. An IPC protocol connecting the desktop UI shell, orchestrator, and sandboxed workers.

Desktop runtime systems have historically suffered severe security vulnerabilities (Zip Slip, memory corruption, use-after-free, buffer overflows) when processing untrusted package inputs in C/C++. We must choose a systems programming language that guarantees memory safety without sacrificing native execution performance.

## Decision drivers

- **Memory safety:** Elimination of buffer overflows, use-after-free, and data races in untrusted archive extraction and binary metadata parsing.
- **Type system & correctness:** Strong static typing, algebraic data types (enums), and explicit error handling (`Result<T, E>`) for robust lifecycle state machine transitions.
- **Cross-platform concurrency:** Ergonomic async I/O and process supervision across Windows, macOS, and Linux.
- **Modular interface contracts:** Rust traits provide a clean, zero-cost abstraction for backend contracts (`IAndroidBackend`, `IIosBackend`).
- **Decoupling from presentation:** Clear process and IPC separation between the UI shell and the runtime engine.

## Options considered

1. **C++20 across all components:**
   - *Pros:* Native integration with Qt 6; existing C++ ecosystem for hypervisors.
   - *Cons:* Inherent memory safety risks in archive decompression and untrusted binary parsing; complex concurrency and error propagation.
2. **Go:**
   - *Pros:* Fast compilation, simple concurrency.
   - *Cons:* Garbage collection overhead; awkward C/C++ FFI integration with platform hypervisors; weaker trait/polymorphism model for low-level systems.
3. **Rust for Core Runtime with Decoupled UI (Selected):**
   - Package intake worker, lifecycle orchestrator, and backend contracts written in Rust.
   - Platform-specific native adapters written in Rust, leveraging narrow C/C++ FFI only where necessary (e.g. interfacing with WHPX, Virtualization.framework, or C libraries).
   - Desktop UI developed in Qt 6 (or cross-platform framework) as a thin presentation shell communicating via versioned, schema-validated IPC.

## Decision

Adopt Option 3. Rust is the core implementation language for:
- `umr-intake-worker`: Isolated binary handling untrusted archives and metadata extraction.
- `umr-orchestrator`: Core engine managing session lifecycles, backend capability negotiation, and event logging.
- `umr-core`: Shared types, domain models, error taxonomy, and backend traits.
- Versioned, schema-validated IPC (JSON-RPC or Protobuf over standard pipes/domain sockets) connects the orchestrator to the desktop shell.

## Consequences

### Positive
- Strict memory safety by default across the untrusted input attack surface.
- Explicit, compiler-verified state machines prevent illegal lifecycle transitions.
- High-performance, low-overhead native binaries on all three desktop operating systems without runtime VM/GC overhead.
- Clean architectural boundary: UI process cannot load untrusted guest native code or crash the orchestrator.

### Negative / trade-offs
- Interfacing with certain native C/C++ platform SDKs requires FFI bindings (e.g. `windows-rs` for WHPX, Objective-C/Swift bridge for Apple frameworks).
- Team members must be proficient in Rust idioms, ownership semantics, and async runtime patterns.

### Risks and mitigations
- *Risk:* FFI overhead and unsafe blocks for hypervisor interop.
- *Mitigation:* Confine `unsafe` code strictly to isolated host platform adapter crates with narrow, safe Rust wrapper interfaces.

## Revisit when

- A core platform requirement necessitates an in-process monolithic architecture incompatible with Rust/IPC boundaries.

