# Stable Project Context for Agents

UMR (working name) is a desktop-first platform intended to run supported Android and iOS applications on Windows, macOS, and Linux.

## Round 1 target

- Hosts: Windows, macOS, Linux.
- App families: Android and iOS are both explicit goals.
- UI: one unified desktop shell.
- Runtime: separate backends, with host-specific adapters as required.
- Android: practical initial execution workstream; choose per-host guest/runtime strategy after experiments.
- iOS: major technical risk; use a feasibility gate and document exactly which apps run, where they run, and by what method.
- Deferred: mobile host platforms, public store, universal all-app compatibility claims.

## Non-negotiable truths

- CPU translation does not provide missing OS or framework APIs.
- Parsing APK/IPA/Mach-O is not the same as executing the app.
- Graphics mapping does not guarantee semantic compatibility or GPU feature parity.
- Remote execution is not local execution and must be labelled accordingly.
- Every support claim must map to reproducible tests and an entry in `docs/05-platform-support-matrix.md`.
- Imported packages and their metadata are untrusted.
- Third-party code/images/apps require license and redistribution review.

## Engineering defaults (provisional)

- Keep the desktop shell, orchestrator, host adapters, and app-family backends separated.
- Prefer reuse of mature components after technical, security, and license evaluation.
- Make small vertical slices; avoid speculative stubs and empty frameworks.
- Choose languages/UI/build systems only after the Phase 0 feasibility and team-capability review. C++20/Rust and Qt 6 are candidates, not irrevocable decisions.

## First desired result

A feasibility evidence pack plus a minimal working vertical slice that launches real, authorized test apps and accurately reports failures/limitations.
