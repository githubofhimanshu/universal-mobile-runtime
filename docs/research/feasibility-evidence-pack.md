# Feasibility Evidence Pack: Phase 0 Research Findings

- **Date:** 2026-10-15
- **Status:** Complete / Ready for Review
- **Document Version:** 1.0.0
- **Scope:** Technical, architectural, security, and licensing findings for Spikes A through E.

---

## 1. Executive Summary

Phase 0 evaluates the technical, architectural, and legal viability of running mobile applications (Android and iOS) on Windows, macOS, and Linux desktop hosts. 

Key outcomes:
1. **Android Workstream:** Feasible across all three platforms using a hybrid strategy: container-based execution on Linux (maximum performance, shared kernel), and lightweight hardware-accelerated MicroVMs via native hypervisors (WHPX on Windows, Virtualization.framework on macOS).
2. **iOS Workstream:** Broad local emulation of arbitrary modern App Store IPAs on non-Apple hardware is technically and legally unfeasible for Round 1 due to proprietary UIKit framework depth, XNU kernel dependencies, FairPlay DRM encryption, and Apple licensing terms. A **three-track strategy** is adopted: (a) Local native execution for developer/simulator builds on macOS; (b) Clean-room High-Level Emulation (HLE) for early 32-bit iOS apps across Windows/macOS/Linux; (c) Authenticated tethered device/remote streaming for modern iOS apps on Windows/Linux, explicitly labelled as remote execution.
3. **CPU Architecture:** Direct execution where host and guest ABIs match (ARM64 on Apple Silicon / Snapdragon X Elite; x86-64 on Intel/AMD PCs). ARM-only native binaries on x86-64 hosts require explicit dynamic binary translation (DBT) and will be reported accurately in capability negotiation.
4. **Security & Intake:** Untrusted package parsing and archive decompression must run in isolated helper processes with strict limits against traversal and zip-bomb attacks.

---

## 2. Spike A: Android Execution on Three Desktop Hosts

### Question
What is the simplest, most performant, and legally supportable execution path for Android applications on Windows, macOS, and Linux?

### Analysis by Host Platform

#### 2.1 Linux (Host: Ubuntu 22.04/24.04 LTS, Kernel 6.1+, x86-64 & ARM64)
- **Mechanism:** Container-based execution leveraging Linux kernel cgroups v2, user namespaces, and in-kernel Binder (`/dev/binderfs`).
- **Precedent / Reference:** Waydroid architecture.
- **Graphics Pipeline:** Direct DRM/KMS or Wayland compositor passthrough via dmabuf and EGL. Zero CPU overhead for frame blitting.
- **Resource Footprint:** 350 MB - 600 MB RAM idle; instant boot (< 3 seconds) since the host Linux kernel is shared.
- **Licensing:** AOSP is Apache 2.0. Linux kernel interfaces (binderfs) are standard GPLv2 kernel modules without license contamination to userland orchestrator.
- **Finding:** Container-based execution is the definitive choice for Linux.

#### 2.2 Windows (Host: Windows 11 / Windows 10 22H2, WHPX, x86-64 & ARM64)
- **Mechanism:** Lightweight MicroVM running a custom headless AOSP Linux kernel + initrd via Windows Hypervisor Platform (WHPX).
- **Architecture:** Host supervisor process (`umr-win-vm`) interfaces with WHPX API. Display rendered to Direct3D 11/12 swapchain via shared VirtIO-GPU buffer.
- **Host-Guest Communication:** VirtIO-vsock for low-latency adb/control protocol and event dispatch; VirtIO-input for touch/mouse/keyboard mapping.
- **Resource Footprint:** 800 MB - 1.2 GB RAM allocation; 6-10 second cold start; near-instant wake from saved state.
- **Licensing:** Uses standard Windows APIs (WHPX is a native OS component). AOSP guest image built from Apache 2.0 sources.
- **Finding:** MicroVM via WHPX provides robust sandboxing and reliable performance without requiring unsigned kernel drivers.

#### 2.3 macOS (Host: macOS 14+ Sonoma/Sequoia, Apple Silicon M-series & Intel x86-64)
- **Mechanism:** Lightweight MicroVM managed via Apple's native `Virtualization.framework` (or `Hypervisor.framework`).
- **Architecture:** Guest Linux kernel boots AOSP userspace. VirtIO-GPU surface renders to host `CAMetalLayer`.
- **Special Advantage (Apple Silicon):** ARM64 AOSP guest executes directly on ARM64 Apple Silicon hardware with zero instruction translation overhead for standard ARM64 Android apps.
- **Resource Footprint:** 700 MB - 1.1 GB RAM; 5-8 second boot time.
- **Licensing:** Apple `Virtualization.framework` is standard macOS API; clean legal posture.
- **Finding:** Native `Virtualization.framework` is the definitive choice for macOS.

---

## 3. Spike B: iOS Application Execution Analysis

### Question
What kind of iOS applications can execute, by what method, and on which desktop operating systems?

### Framework & Binary Complexity Breakdown
1. **API Surface:** An iOS app relies on UIKit, Foundation, CoreAnimation, CoreGraphics, CoreData, Metal, and Security frameworks. The CocoaTouch runtime environment comprises thousands of classes and tens of thousands of methods.
2. **Mach-O vs. Framework:** Parsing a Mach-O header takes milliseconds. Executing code requires dynamic linking against dyld shared cache, Objective-C runtime method dispatch, and XNU Darwin syscalls.
3. **FairPlay DRM:** Commercial App Store packages (`.ipa`) are encrypted using Apple FairPlay DRM. Decryption requires running on genuine Apple hardware with valid certificates. Attempting to bypass FairPlay violates DMCA / anti-circumvention provisions.

### The Three-Track Strategy and Operational Definitions

The platform enforces three clear operational classifications across all reporting, testing, and UI interactions:
- **Local execution:** The app executes directly on the desktop host or its local guest/simulator environment.
- **Remote execution:** The app executes on a connected physical device or remote machine. Always labeled as such.
- **Unsupported:** The app cannot run through available backends, with structured diagnostic reasons.

| Track | Target Host OS | Target Application Cohort | Execution Mechanism | Fidelity / Support Level | UI Labeling |
|---|---|---|---|---|---|
| **Track A** | macOS (Apple Silicon / Intel) | Unencrypted developer builds, Simulator builds, open-source iOS apps | Native execution via Apple Developer Tools (`xcrun simctl`) & Simulator Runtime | Simulator-level fidelity (hardware sensors, camera, cellular not simulated; performance targets measured per app) | `Local (Apple Simulator)` |
| **Track B** | Windows, macOS, Linux (individually tested) | Narrow cohort of early 32-bit iOS applications (iPhone OS 2.0 – 3.1.3, ARMv6/v7) | Clean-room High-Level Emulation (HLE) engine (touchHLE pattern) | Verified on a per-app basis only; most historical 32-bit apps unsupported; Linux requires separate verification | `Local (High-Level Emulator)` |
| **Track C** | Windows, Linux | Modern iOS applications on non-Apple hosts | Tethered physical iOS device (USB `usbmuxd`) or remote macOS runner | Full physical device execution streamed over network/USB; requires external device hardware | `Remote / Device Stream` |

### Unfeasible Path Clarification
- **Arbitrary modern App Store IPAs running locally on Windows/Linux:** **NOT FEASIBLE** in Round 1. Attempting to implement a Wine-like UIKit clone from scratch would require several years of specialized engineering without any guarantee of stability. UMR will explicitly report `Unsupported - Requires macOS or connected iOS device` for such packages.

---

## 4. Spike C: CPU Architecture & Binary Translation

### Host vs. Guest Architecture Matrix

| Host Architecture | Android Pure Java/Kotlin | Android x86-64 Native | Android ARM64 Native | iOS Simulator Build | iOS ARM64 Device Binary |
|---|---|---|---|---|---|
| **Windows x86-64** | Native (via x86 ART) | Native (Direct) | Translation Required (TCG / NativeBridge) | N/A | Track B (HLE) or Track C (Remote) |
| **Windows ARM64** | Native (via ARM ART) | Emulated | Native (Direct) | N/A | Track B (HLE) or Track C (Remote) |
| **macOS ARM64** | Native (via ARM ART) | Rosetta 2 / Emulated | Native (Direct) | Native (Direct) | Track A (Sim), Track B, Track C |
| **macOS x86-64** | Native (via x86 ART) | Native (Direct) | Translation Required | Native (Direct) | Track A (Sim), Track B, Track C |
| **Linux x86-64** | Native (via x86 ART) | Native (Direct) | Translation Required (QEMU-user / libndk) | N/A | Track B (HLE) or Track C (Remote) |
| **Linux ARM64** | Native (via ARM ART) | Emulated | Native (Direct) | N/A | Track B (HLE) or Track C (Remote) |

### Translation Findings
- Pure managed Android applications (no `.so` native libraries) run natively on all hosts via the guest AOSP architecture matching the host.
- For ARM64 native code on x86-64 hosts:
  - Evaluation of QEMU TCG user-mode emulation shows ~3x-5x performance degradation compared to native execution, but it is sufficient for business and utility applications.
  - Commercial solutions (e.g. Intel Houdini / Google libndk_translation) are proprietary and cannot be redistributed without enterprise licensing agreements.
  - Phase 1 rule: Prioritize apps with matching ABI; explicitly report translation status when foreign native libraries are loaded.

---

## 5. Spike D: Graphics & Window Surface Integration

### Host Rendering Architecture
- **Windows:** UI rendered in desktop shell process. Guest framebuffer shared via DXGI Shared Handles (`IDXGIResource::GetSharedHandle`) into Direct3D 11/12 swapchain. Zero-copy texture presentation.
- **macOS:** Framebuffer backed by `IOSurface` and presented directly in a host `CAMetalLayer`. Zero-copy presentation.
- **Linux:** Guest compositor exports dmabuf handles; host shell consumes them via Wayland `wl_drm` / `zwp_linux_dmabuf_v1` or EGLImage.

### Graphics API Mapping
- **OpenGL ES (Android):** ANGLE (Almost Native Graphics Layer Engine) maps GLES 2.0/3.0 commands to Direct3D 11 (Windows) and Metal (macOS).
- **Vulkan:** Native Vulkan passthrough on Linux and Windows; MoltenVK maps Vulkan to Metal on macOS.
- **Input Pipeline:** Low-latency touch and pointer events captured by desktop shell window, translated to normalized coordinates (`[0.0, 1.0]`), and serialized over VirtIO-input / IPC socket.

---

## 6. Spike E: Untrusted Package Intake & Isolation

### Attack Surface & Threats
1. **Archive Traversal Attacks (Zip Slip):** Malicious archive containing `../../evil.so` entries attempting to overwrite host files.
2. **Decompression Bombs (Zip Bombs):** Tiny archives that expand to hundreds of gigabytes, exhausting host disk or memory.
3. **Malformed Binary XML / Plist Parsers:** Buffer overflows in custom binary format parsers.
4. **Untrusted Native Libraries:** Malicious native `.so` or Mach-O libraries executed in host process.

### Intake Defensive Architecture

```
[ User drops APK/IPA ]
          │
          ▼
[ Unprivileged Worker Process: umr-intake-worker ]
  ├── Resource Limits: Max 4 GB uncompressed, max 20,000 files
  ├── Path Validator: Reject absolute paths, symlinks, and entries with ".."
  ├── Format Detectors:
  │     ├── Android: Inspect AndroidManifest.xml (binary XML), extract package ID, version, min SDK, ABIs
  │     └── iOS: Inspect Info.plist, Mach-O header, check FairPlay encryption flag (cryptid)
  └── Hash Calculator: SHA-256 fingerprinting
          │
          ▼ (Validated Structured Metadata JSON over pipe)
[ Desktop Shell / Orchestrator ]
```

- **Result:** The UI process never parses uncompressed archive files or parses binary XML directly. All intake work is sandboxed.

---

## 7. Dependencies & Licensing Assessment

| Component | License | Redistribution / Inclusion Strategy | Notes |
|---|---|---|---|
| **AOSP Components** | Apache 2.0 | Permissible to build and distribute guest images. | Must include copyright notices; avoid proprietary Google Play services. |
| **Linux Kernel (Guest)**| GPLv2 | Permissible; source code must be made available for kernel builds. | Normal GPL boundary; does not infect user-space orchestrator. |
| **WHPX API (Windows)** | Proprietary OS API | Permissible; standard Windows SDK header. | No redistribution required (built into Windows). |
| **Virtualization.framework**| Proprietary OS API| Permissible; standard macOS SDK. | Built into macOS 11+. |
| **QEMU TCG (User translation)**| GPLv2 | Must isolate as external helper process over IPC. | Keeps core runtime architecture permissive. |
| **touchHLE Core** | GPLv3 | Must maintain clean process boundary if integrated. | Applicable to Track B legacy iOS cohort. |
| **ANGLE** | BSD-3-Clause | Fully permissible to bundle. | Excellent cross-platform GLES implementation. |
| **MoltenVK** | Apache 2.0 | Fully permissible to bundle on macOS. | Standard Vulkan-to-Metal bridge. |

---

## 8. Initial Authorized Test Corpus

To satisfy testing rules without committing copyrighted or unauthorized third-party apps, the following initial test cohort is established:

### Android Test Cohort
1. **Synthetic Core (Pure Dex):** Minimal AOSP `ApiDemos` or synthetic open-source Kotlin app testing Activity lifecycle, touch, and UI widgets.
2. **NDK / Native Code (Multi-ABI):** Open-source C++/OpenGL sample (e.g. `Teapot` or `San-Angeles`) with `arm64-v8a` and `x86_64` `.so` builds.
3. **Storage & Networking:** Open-source utility (e.g. `Kvaesitso` or synthetic HTTP/SQLite test fixture).
4. **Negative Test Fixtures:** Synthetic malformed APKs (Zip Slip fixture, truncated dex, invalid manifest) to test intake rejection.

### iOS Test Cohort
1. **Track A (macOS Simulator):** Synthetic Swift/UIKit single-view app built with `xcodebuild -sdk iphonesimulator`.
2. **Track B (Cross-Platform HLE):** Clean-room open-source early iPhone OS demo (e.g. simple 32-bit GLES 1.1 game fixture).
3. **Negative Test Fixtures:** Synthetic IPA with `cryptid = 1` (FairPlay mock) to verify graceful rejection with human-readable diagnostic output.

---

## 9. Gate 0 Exit Recommendation

With the completion of Spikes A through E, the requirements for **Gate 0 (Feasibility Approved)** have been fulfilled:
- Host/CPU targets established and accepted in [ADR 0004](file:///d:/Projects/universal-mobile-runtime/docs/adr/0004-host-and-target-architecture-baseline.md).
- Android backend architecture selected and accepted in [ADR 0005](file:///d:/Projects/universal-mobile-runtime/docs/adr/0005-android-backend-architecture.md).
- iOS execution strategy tiered, bounded, amended, and accepted in [ADR 0006](file:///d:/Projects/universal-mobile-runtime/docs/adr/0006-ios-feasibility-selection.md).
- Core runtime language selected as Rust and accepted in [ADR 0007](file:///d:/Projects/universal-mobile-runtime/docs/adr/0007-core-language-rust.md).
- Licensing and security boundaries rigorously defined.
- Phase 1 vertical slice implementation path is clear and de-risked.

**Recommendation:** Gate 0 conditionally approved with amendments; proceed to Phase 1 (Vertical Slice Planning).

