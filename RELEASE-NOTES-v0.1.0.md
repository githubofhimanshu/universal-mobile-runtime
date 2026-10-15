# Release Notes: Universal Mobile Runtime (UMR) v0.1.0-alpha

**Release Date:** 2026-10-15  
**Milestone:** Round 1 Completion (All Gates Passed)  
**Git Tag:** `v0.1.0-alpha`  
**Repository:** [github.com/githubofhimanshu/universal-mobile-runtime](https://github.com/githubofhimanshu/universal-mobile-runtime)

---

## Executive Summary

The Universal Mobile Runtime (UMR) team is proud to announce the **v0.1.0-alpha** release, marking the successful completion of all **Round 1 Milestones**.

UMR delivers a desktop-first execution platform that allows users to discover, validate, install, run, and inspect mobile applications (**Android and iOS**) on **Windows, macOS, and Linux hosts**. The entire engine is implemented in **100% safe Rust** with zero `unsafe` blocks, strict hypervisor sandboxing, and zero deceptive compatibility claims or mock UIs.

---

## Highlights by Milestone Phase

### Phase 0: System Architecture & Standards Baseline
- Established 8 formal Architecture Decision Records ([ADR 0001–0008](file:///d:/Projects/universal-mobile-runtime/docs/adr/)).
- Published Threat Model and Platform Support Matrix baseline.

### Phase 1: Core Vertical Slice (Gate 1 Passed)
- **Sandboxed Intake Worker:** Isolated process extracting APK/IPA metadata (`AndroidManifest.xml`, `Info.plist`, DEX headers, Mach-O architecture slices) with Zip Slip directory traversal prevention.
- **Runtime Lifecycle Orchestrator:** Explicit lifecycle state machine (`Unavailable`, `Ready`, `Preparing`, `Running`, `Stopping`, `Failed`).
- **Deterministic Reference Backend:** Mock backend providing verifiable lifecycle proofs.

### Phase 2: Android Depth & Desktop Shell (Gate 2 Passed)
- **Subsystems Implementation:**
  - Pointer/pixel coordinate translation with multi-touch and navigation keys (Back, Home, Recents).
  - Storage sandboxing isolating app filesystems (`/data/data/<pkg>` and `/sdcard/`) with clean wipe capabilities.
  - 60 FPS RGBA8888 framebuffer compositor rendering status bar, navigation bar, and touch indicators.
- **Desktop Shell UI:** HTML5/TypeScript presentation shell with interactive phone canvas viewport, drag-and-drop installer, app library, and diagnostics drawer.
- **JSON-RPC 2.0 Gateway:** High-performance loopback RPC server on port `9090`.

### Phase 3: Cross-Host Virtualization & Adapters (Gate 3 Passed)
- **Host Detection:** Multi-host detection of Windows WHPX, Linux Cgroups/Binderfs, and macOS Virtualization.framework.
- **Transport Bridges:** VirtIO-vsock socket bridges and ADB loopback protocol.
- **Persistent Package Manager:** State-preserving application upgrades and clean package uninstallation.
- **Watchdog Supervisor:** Process crash containment and ANR monitoring with signal 11/6 stack extraction.

### Phase 4: Tiered iOS Backend Expansion (Gate 4 Passed)
- **Strict Adherence to ADR 0006 (Zero Fake Mocks):**
  - **Track A (Local macOS Simulator):** Direct integration with Apple Simulator (`xcrun simctl`) for simulator builds, rendering 1170x2532 Retina frames.
  - **Track B (Local Legacy Emulation):** High-Level Emulation (HLE) targeting early 32-bit iPhone OS 2.0–3.1.3 applications on ARMv7.
  - **Track C (Remote Stream):** Authenticated bridge for tethered physical devices and remote macOS runners with mandatory `RemoteStream` UI watermarking.
- **Framework Gap Tracker:** Automated audit detecting linked frameworks (Metal, UIKit, StoreKit, CloudKit) and immediately rejecting Apple FairPlay DRM encrypted binaries (`cryptid != 0`).

### Phase 5: Distribution, Provenance & Ecosystem (Gate 5 Passed)
- **Curated Catalog:** Signed application catalog with verified publisher identities and search filters.
- **Provenance & Anti-Downgrade Verifier:** Cryptographic SHA-256 binary hashing, signature verification, and strict rollback prevention (`new_version_code >= current_version_code`).
- **Licensing Compliance:** Mandatory user EULA acceptance ledger and open-source license attribution notices.
- **Remote Privacy Gate:** Mandatory user consent required before starting Track C remote execution sessions.
- **Redacting Support Reports:** Diagnostic bundle exporter with automated scrubbing of auth tokens, passwords, private keys, email addresses, and home directory paths.

---

## Test & Quality Verification

```
Full Workspace Test Suite:
├── umr-backend-android:     7 passed
├── umr-backend-ios:         8 passed
├── umr-backend-reference:   6 passed
├── umr-core:               12 passed
├── umr-distribution:       17 passed (9 unit + 8 integration)
├── umr-intake-worker:       5 passed
├── umr-ipc:                10 passed (9 unit + 1 e2e)
└── umr-orchestrator:       10 passed (2 unit + 7 smoke + 1 fixtures)
Total: 75 passed; 0 failed; 0 warnings; execution time: 1.48s
```

---

## Workspace Crates

| Crate | Purpose |
| :--- | :--- |
| `umr-core` | Core domain models, state machine, error taxonomy, framebuffer traits |
| `umr-intake-worker` | Sandboxed archive parser and binary inspector |
| `umr-orchestrator` | Lifecycle supervisor and backend capability negotiator |
| `umr-backend-reference` | Deterministic reference backend for testing |
| `umr-backend-android` | Android virtualization adapters, VirtIO transport, and package manager |
| `umr-backend-ios` | Tiered iOS execution backends and framework gap analysis |
| `umr-distribution` | Curated catalog, provenance verifier, privacy gate, and support scrubber |
| `umr-ipc` | JSON-RPC 2.0 daemon and HTTP loopback server |

---

## Next Steps: Roadmap Post-Round 1
- Native GPU passthrough (Vulkan/Metal zero-copy frame sharing).
- Cross-architecture dynamic binary translation for ARM64 apps on x86-64 hosts.
- Cloud execution orchestrator for enterprise multi-tenant runners.
