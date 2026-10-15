# Feature Specification: Phase 1 Minimal End-to-End Vertical Slice

- **Status:** Draft / Ready for Review
- **Owner:** System Architect & Core Engineering Lead
- **Related requirement(s):** G1, G2, G4, G5, FR-UI-01..04, FR-PKG-01..04, FR-RUN-01..05, FR-COMP-01..03, NFR-SEC-01, NFR-REL-01, NFR-TEST-01
- **Related ADR(s):** [ADR 0001](file:///d:/Projects/universal-mobile-runtime/docs/adr/0001-desktop-first.md), [ADR 0002](file:///d:/Projects/universal-mobile-runtime/docs/adr/0002-multiple-backends.md), [ADR 0003](file:///d:/Projects/universal-mobile-runtime/docs/adr/0003-ios-feasibility-gate.md), [ADR 0004](file:///d:/Projects/universal-mobile-runtime/docs/adr/0004-host-and-target-architecture-baseline.md), [ADR 0005](file:///d:/Projects/universal-mobile-runtime/docs/adr/0005-android-backend-architecture.md), [ADR 0006](file:///d:/Projects/universal-mobile-runtime/docs/adr/0006-ios-feasibility-selection.md), [ADR 0007](file:///d:/Projects/universal-mobile-runtime/docs/adr/0007-core-language-rust.md)

---

## 1. Problem

Currently, Universal Mobile Runtime is a documentation-only repository with zero runtime implementation. To establish the platform without falling into the trap of broad, speculative scaffolding or unvetted monolithic dependencies, we need a small, robust, testable, end-to-end vertical slice. 

The vertical slice must prove the core architectural thesis:
1. Untrusted packages (APK/IPA) can be ingested and validated safely in an isolated Rust worker process without risking host or UI compromise.
2. The lifecycle orchestrator can negotiate backend capabilities and manage deterministic transitions (`Unavailable` -> `Ready` -> `Preparing` -> `Running` -> `Stopping` -> `Failed`).
3. An explicit execution backend can launch, supervise, stop, and collect logs from a real authorized mobile test application on a reference host.
4. Unsupported packages (such as encrypted App Store IPAs or missing local backends) produce structured, actionable diagnostic errors rather than unhandled panics or deceptive mocks.

---

## 2. Outcome

When Phase 1 is complete, an operator can invoke the core runtime binary via CLI/IPC to:
1. Submit an APK or IPA package path.
2. Receive validated metadata (package ID, version, architecture/ABIs, SDK levels, SHA-256 hash) or an immediate structured rejection (e.g. traversal attempt, zip bomb, encrypted binary).
3. Have the orchestrator automatically select the compatible execution backend.
4. Execute the application lifecycle on the reference host (prepare -> launch -> poll status -> clean shutdown -> log retrieval).
5. For an iOS package on non-macOS or unsupported modern IPA, receive an explicit structured outcome classifying the package as requiring a remote/tethered device or unsupported, adhering to Gate 0 definitions.

---

## 3. Scope

### In Scope
- **Rust Workspace Setup:** Cargo workspace layout (`crates/umr-core`, `crates/umr-intake-worker`, `crates/umr-orchestrator`, `crates/umr-backend-api`, `crates/umr-android-reference`).
- **Sandboxed Package-Intake Worker:**
  - Strict archive validation (file size <= 4 GB, file count <= 20,000, compression ratio <= 100:1).
  - Path canonicalization rejecting `..`, absolute paths, and symbolic links.
  - Safe parsing of `AndroidManifest.xml` (binary XML) to extract package name, version, min SDK, and native ABIs (`lib/*`).
  - Safe parsing of iOS `Info.plist` and Mach-O headers to extract bundle ID, version, target architecture, and FairPlay encryption status (`cryptid`).
  - SHA-256 digest computation.
- **Lifecycle Orchestrator:**
  - Deterministic state machine with timeout enforcement and resource cleanup.
  - Backend capability discovery and negotiation engine.
  - Structured event streaming and redacted diagnostic log collection.
- **Backend Traits & Contracts:**
  - Rust traits defining `IAndroidBackend` and `IIosBackend`.
  - Capability descriptors (`ExecutionMode`: `LocalNative`, `LocalVirtualized`, `LocalTranslated`, `LocalHle`, `RemoteStream`).
- **Reference Android Execution Proof:**
  - End-to-end lifecycle proof on reference host using an authorized, synthetic Android test package.
- **Comprehensive Automated Tests:** Unit tests for parsers, integration tests for IPC/orchestrator, negative security tests for malformed archives.

### Out of Scope
- Full desktop GUI application (Qt 6 desktop shell is decoupled and will connect via IPC in Phase 2).
- Dynamic binary translation engines (QEMU TCG integration deferred to subsequent phases).
- General-purpose graphics compositing or Direct3D/Metal swapchain surfaces (headless/off-screen or window-stub lifecycle in Phase 1).
- Full iOS UIKit reimplementation or touchHLE embedding.
- Public app store, billing, or multi-tenant cloud streaming.

---

## 4. End-to-End Workflow Architecture

```
User / CLI / Future UI
        │ (Selects APK or IPA path)
        ▼
[ Rust Package-Intake Worker (Subprocess) ]
        │
        ├─► [ Validation Checks ]
        │     ├── Archive size <= 4GB, entries <= 20k, ratio <= 100:1
        │     ├── Path traversal sanitization (no '..', no symlinks)
        │     ├── Format integrity (DEX header / Mach-O header)
        │     └── FairPlay DRM check (cryptid == 0)
        │
        ├── [ Rejected ] ──► Return structured DiagnosticError
        │                     (e.g., ERR_ZIP_TRAVERSAL, ERR_DRM_PROTECTED)
        │
        ▼ [ Accepted ]
Structured Metadata (JSON over IPC)
  (Package ID, Version, ABIs, MinSDK, SHA-256)
        │
        ▼
[ Lifecycle Orchestrator ]
        │
        ▼
[ Backend Capability Negotiation ]
        │
        ├── [ No Compatible Local Backend ] ──► Return Explicit Unsupported /
        │                                       Remote-Required Result
        │
        ▼ [ Compatible Android Backend Found ]
[ Android Reference Backend ]
        │
        ├── Prepare (Sandbox & Resource Allocation)
        ├── Launch Application (Supervised Child Process / Runner)
        ├── Monitor Lifecycle (Heartbeat, State: Running)
        ├── Stop (SIGINT/SIGTERM, Timeout Enforcement)
        └── Collect Diagnostics (Sanitized Logs, Resource Release)
```

---

## 5. Acceptance Criteria

- [ ] **AC-1 (Intake Safety):** Malformed, path-traversing, or zip-bomb archives are rejected immediately with structured error codes without crashing or hanging the intake worker.
- [ ] **AC-2 (Metadata Fidelity):** For valid APKs, the intake worker accurately reports package ID, version code, version name, min SDK level, and all embedded native ABIs (`arm64-v8a`, `x86_64`, etc.).
- [ ] **AC-3 (IPA Classification):** For iOS IPAs, the intake worker accurately reports bundle ID, target architectures, and flags FairPlay encryption (`cryptid != 0`) as `Unsupported - DRM Protected`.
- [ ] **AC-4 (Orchestrator Lifecycle):** The orchestrator transitions through explicit states (`Unavailable` -> `Ready` -> `Preparing` -> `Running` -> `Stopping` -> `Terminated`) and handles process timeouts gracefully.
- [ ] **AC-5 (Capability Negotiation):** If an app requires capabilities not present on the host (e.g. ARM-only native library on x86 host without translator, or modern iOS IPA on non-macOS host), the orchestrator returns an explicit `IncompatibleBackend` diagnostic rather than attempting a doomed launch.
- [ ] **AC-6 (Execution Proof):** A real authorized synthetic Android APK executes successfully through prepare -> launch -> stop -> log capture on the reference host environment, and leaves zero zombie processes upon shutdown.
- [ ] **AC-7 (Log Sanitization):** Collected diagnostic outputs contain no user secrets or environment credentials, adhering to project redaction policy.

---

## 6. Host and Runtime Execution Matrix (Phase 1)

| Host OS | Host CPU | Test Package Type | Expected Outcome | Test Type |
|---|---|---|---|---|
| **Windows 11** | x86-64 | Synthetic Pure-Dex APK | Full lifecycle pass (Prepare -> Run -> Stop) | Integration Smoke |
| **Windows 11** | x86-64 | Malformed Zip Slip APK | Rejected with `ERR_ZIP_TRAVERSAL` | Security Negative Unit |
| **Windows 11** | x86-64 | Encrypted iOS Store IPA | Rejected with `ERR_DRM_PROTECTED` | Security Negative Unit |
| **Windows 11** | x86-64 | iOS Simulator IPA | Classified as `RemoteOrMacOSRequired` | Negotiation Unit |
| **Linux (CI)** | x86-64 | Synthetic Pure-Dex APK | Full lifecycle pass | CI Integration |
| **macOS (CI)** | ARM64 | Synthetic Pure-Dex APK | Full lifecycle pass | CI Integration |

---

## 7. Failure Modes & Mitigation

1. **Intake Process Hang (Decompression loop):**
   - *Mitigation:* The orchestrator spawns the intake worker with an OS-enforced job timeout (5 seconds). If exceeded, the process is terminated via `SIGKILL`/`TerminateProcess`.
2. **Backend Child Process Crash:**
   - *Mitigation:* The orchestrator listens for process exit events; if the backend process crashes, state transitions directly to `Failed(exit_code)` with collected stderr preserved. The orchestrator never panics or aborts.
3. **App Hang on Shutdown:**
   - *Mitigation:* Two-stage teardown: Send graceful termination request (SIGTERM); after a 3-second grace period, send forced kill (SIGKILL). All child process handles are tracked in an RAII supervisor.

---

## 8. Proposed Crate & Module Structure

```text
crates/
├── umr-core/                   # Domain models, lifecycle states, error taxonomy, traits
│   ├── src/
│   │   ├── lifecycle.rs        # State machine & transition logic
│   │   ├── metadata.rs         # Package metadata schemas
│   │   ├── error.rs            # Structured, serialized error codes
│   │   ├── backend.rs          # IAndroidBackend & IIosBackend traits
│   │   └── lib.rs
│   └── Cargo.toml
├── umr-intake-worker/          # Standalone sandboxed CLI & parser
│   ├── src/
│   │   ├── archive.rs          # Defensive zip extractor with quotas
│   │   ├── android_manifest.rs # Safe binary XML parser
│   │   ├── ios_plist.rs        # Safe binary/XML plist parser
│   │   ├── macho.rs            # Mach-O header & encryption parser
│   │   └── main.rs
│   └── Cargo.toml
├── umr-orchestrator/           # Lifecycle coordinator & capability negotiator
│   ├── src/
│   │   ├── supervisor.rs       # Process lifetime and timeout manager
│   │   ├── negotiator.rs       # Capability matching algorithm
│   │   ├── session.rs          # Active runtime session state
│   │   └── main.rs             # CLI / IPC entry point
│   └── Cargo.toml
└── umr-backend-reference/      # Reference execution runner implementation
    ├── src/
    │   ├── runner.rs           # Reference execution lifecycle
    │   └── lib.rs
    └── Cargo.toml
```

---

## 9. Next Steps (Implementation Readiness)

Upon approval of this Feature Specification:
1. Initialize the root `Cargo.toml` workspace.
2. Implement `umr-core` types, state machine, and error definitions.
3. Implement `umr-intake-worker` with unit tests for safe archive extraction and manifest parsing.
4. Implement `umr-orchestrator` capability negotiation and session supervisor.
5. Create synthetic test fixtures in `tests/fixtures/`.
6. Implement the reference execution backend and run the end-to-end smoke test suite.

