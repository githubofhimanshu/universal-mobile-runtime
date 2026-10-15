# Product Requirements

**Status:** Draft baseline for discovery and feasibility. **MUST** requirements become release blockers once approved.

## Goals

- G1: Provide one desktop interface for app discovery, import, launch, stop, removal, and diagnostics.
- G2: Run a tested set of Android applications on Windows, macOS, and Linux.
- G3: Establish and demonstrate an iOS application execution path for supported app classes on Windows, macOS, and Linux.
- G4: Report support by host OS, host CPU architecture, app CPU architecture, backend version, graphics backend, and app capabilities.
- G5: Isolate execution backends so the UI does not depend on a specific emulator or translation engine.

## User stories

- As a user, I can import an app package I am authorized to use and receive a clear validation result.
- As a user, I can see whether an app is supported on this computer before attempting to launch it.
- As a user, I can launch and stop a supported app, inspect logs, and understand actionable failures.
- As a user, I can remove an imported package and its UMR-managed data.
- As a developer, I can reproduce a compatibility report with the same host/backend/app versions.
- As an operator, I can update or disable a backend without changing the common application shell.

## Functional requirements

### Application shell

- FR-UI-01: Show installed/imported apps with their package identity, icon where available, and support status.
- FR-UI-02: Provide import, install, launch, stop, uninstall/remove, and diagnostic actions.
- FR-UI-03: Explain whether an execution session is local, virtualized, or remote.
- FR-UI-04: Report unsupported CPU, OS, graphics, permissions, or framework requirements in human-readable form.

### Package intake

- FR-PKG-01: Treat package files and extracted metadata as untrusted.
- FR-PKG-02: Validate supported format and size limits before handing data to a backend.
- FR-PKG-03: Keep package identity, source, hash, detected architecture, and import time in metadata.
- FR-PKG-04: Do not bypass signature, entitlement, or anti-tamper requirements. Support only legally and technically valid workflows.
- FR-PKG-05: Do not automatically redistribute imported apps to a store or another user.

### Runtime lifecycle

- FR-RUN-01: Each backend reports its capabilities before launch.
- FR-RUN-02: Use explicit lifecycle states: unavailable, ready, preparing, running, stopping, failed.
- FR-RUN-03: Capture structured lifecycle events and diagnostics without collecting app secrets by default.
- FR-RUN-04: Stop child processes and release resources when the session ends.
- FR-RUN-05: A backend crash must not crash the UI if process isolation can reasonably prevent it.

### Compatibility reporting

- FR-COMP-01: Record a compatibility result against the exact host, app build/hash, backend version, and configuration.
- FR-COMP-02: Distinguish verified, partial, unsupported, not tested, and blocked by external requirements.
- FR-COMP-03: Never infer compatibility solely from a package parsing successfully.

## Non-functional requirements

- NFR-SEC-01: Import/parsing and runtime processes should use least privilege and isolation appropriate to the host.
- NFR-REL-01: A failed app launch should leave the shell responsive and provide a diagnostic identifier.
- NFR-OBS-01: Logs should be structured, redact secrets by default, and support user-consented export.
- NFR-PORT-01: Platform-specific code must be behind host/backend interfaces where practical.
- NFR-TEST-01: Every supported combination must have automated smoke tests where automation is possible and documented manual tests otherwise.
- NFR-LIC-01: Third-party dependencies and bundled runtime artifacts require a license and redistribution review.

## Product success metrics (initial targets to set after baseline measurement)

- First-launch success rate on a published smoke-test corpus.
- Median and p95 app startup time by host/backend.
- Session crash rate and shell crash rate.
- Percentage of failures with actionable diagnostics.
- Compatibility results reproducible by another developer.
- Number of verified app/host/backend combinations, reported alongside tested corpus size.

Do not set performance targets until the first reference machine and test corpus have been selected.
