# ADR 0006: iOS Feasibility Selection and Tiered Execution Strategy

- **Status:** Accepted (Amended with Gate 0 corrections)
- **Date:** 2026-10-15
- **Decision owners:** Project Lead, iOS Architecture Lead, System Architect

## Context

Per ADR 0003, broad local iOS execution across all desktop platforms presents significant technical and legal challenges:
1. **Framework Breadth:** UIKit, Foundation, CoreGraphics, CoreAnimation, CoreData, Metal, and associated private frameworks represent an immense surface of proprietary Apple code.
2. **Binary Encryption & DRM:** Commercial App Store packages (`.ipa`) are encrypted with Apple FairPlay DRM and target iOS ARM64 physical device hardware, requiring hardware decryption keys not present on desktop computers.
3. **Licensing & Legal Boundaries:** Apple EULAs restrict macOS and the iOS SDK/Simulator to Apple-branded hardware.
4. **Feasibility Gate:** Round 1 demands that we do not misrepresent package parsing or remote streaming as local native execution, and that we provide a verifiable path for a clearly scoped cohort.

## Decision drivers

- Strict compliance with project rules against fake mocks and misleading support claims.
- Accurate distinction between local desktop execution, legacy emulation, and external device streaming.
- Legal constraints around redistribution of Apple proprietary frameworks and FairPlay bypasses.
- Providing genuine utility to developers and QA engineers in Round 1.

## The Acceptance Rule and Execution Definitions

To ensure clarity and prevent misleading claims, the platform adheres to three strictly defined operational states:
- **Local execution:** The application executes directly on the desktop host or its local guest/simulator environment.
- **Remote execution:** The application executes on a connected physical device or remote host. This is never presented or labeled as local execution.
- **Unsupported:** The requested application cannot run through the available local or remote backends, and actionable diagnostic reasons are provided.

## Options considered

1. **Attempt full local CocoaTouch/UIKit reimplementation on Windows/Linux (Wine-for-iOS):**
   - *Result:* Unfeasible for Round 1. Similar efforts (e.g. Darling for macOS, Chameleon, early iOS emulators) took many years and only covered a tiny fraction of UIKit/XNU APIs. Modern apps would immediately crash.
2. **Claim iOS is out of scope and drop it entirely:**
   - *Result:* Violates the explicit Round 1 product commitment to include both app families.
3. **Tiered Execution Strategy with Explicit Capability Boundaries (Selected):**
   - Partition iOS execution into three distinct, transparently labelled runtime paths:
     - **Track A (Local Execution on macOS):** Run applications through Apple Developer tools (`xcrun simctl` / Xcode simulator runtime). The imported package must be specifically built for the simulator runtime (or Mac Catalyst / Designed for iPad on Mac where compatible); an arbitrary device-targeted `.ipa` will not run in the simulator. Simulated devices do not reproduce every hardware feature (camera, specialized sensors, Bluetooth peripherals, cellular) or physical-device performance characteristics. Performance targets must be measured and tested rather than assumed.
     - **Track B (Local Legacy iOS Emulation - Windows, macOS, Linux):** High-Level Emulation (HLE) engine (leveraging clean-room approaches like touchHLE) for a tightly defined cohort of early 32-bit iOS applications (iPhone OS 2.0-3.1.3, ARMv6/v7, OpenGL ES 1.1/2.0, OpenAL, early UIKit). Windows, macOS, and Linux are treated as individually tested targets. Support is strictly per-app verified cohort; most historical 32-bit applications do not work, and Linux requires separate build and test verification.
     - **Track C (Remote Modern iOS Execution - Windows, Linux Hosts):** For modern iOS applications on non-macOS desktops, establish an authenticated bridge to a supported physical iOS device (via USB `usbmuxd` / WebDriverAgent / ScreenCaptureKit) or a remote Mac running the application. Provide display streaming and supported interaction mechanisms. This must **always be labelled "Remote / Device Stream"** in the product, UI, logs, and documentation. Execution on a remote device is not local execution on the desktop.

## Decision

Adopt Option 3. Define the `IIosBackend` abstraction to support these three distinct provider tracks. The UI shell must display the exact execution mode (`Local Simulator`, `Local High-Level Emulator`, or `Remote / Device Stream`). Unencrypted modern IPAs on Windows/Linux that require missing UIKit frameworks will be flagged as `Unsupported - Missing Framework APIs` instead of attempting execution and failing silently.

## Consequences

### Positive
- Fully satisfies Gate 0 / Gate 1 feasibility criteria with demonstrable, real execution.
- Respects legal boundaries and avoids implementing illegal DRM bypasses.
- Provides immediate real-world value to macOS developers testing simulator packages and cross-platform users running verified HLE apps or tethered test devices.
- Sets honest expectations for users without marketing deception.

### Negative / trade-offs
- Modern iOS apps cannot run locally on Windows or Linux without external hardware or macOS test hosts.
- The HLE track on Windows/Linux is limited to early-era applications and requires individual verification.

### Risks and mitigations
- *Risk:* User disappointment that modern App Store games do not run locally on Windows.
- *Mitigation:* Clear intake diagnostics: When an `.ipa` is inspected, detect minimum iOS version, architecture, and encryption status. If encrypted or targeting modern UIKit without a connected device, immediately provide actionable diagnostic feedback explaining why local execution is unavailable.

## Revisit when

- Viable open-source modern UIKit/XNU compatibility layers emerge with verified test suites.
