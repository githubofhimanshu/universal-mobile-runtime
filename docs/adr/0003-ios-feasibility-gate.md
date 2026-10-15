# ADR 0003: iOS Compatibility Requires an Explicit Feasibility Gate

- **Status:** Proposed
- **Date:** 2026-10-15
- **Decision owners:** Project owner and engineering lead

## Context

Loading an IPA or Mach-O file does not recreate the iOS execution environment. App behavior can depend on system frameworks, device services, signing and entitlements, CPU architecture, graphics APIs, and external Apple services. The availability of an iOS execution approach may differ between Windows, macOS, and Linux.

## Decision

Before committing to full iOS implementation, produce evidence for a defined app cohort and a clearly labelled method on every intended host: local emulation, supported Apple development/simulator workflow, remote device execution, or another reviewed approach. Local and remote execution are separate product modes.

Round 1 cannot be declared complete for an advertised iOS host combination without running and testing real applications through the declared approach. If the requested local-execution goal cannot be met, the project owner must explicitly revise scope or hold release.

## Consequences

- A feasibility spike is an early deliverable, not optional research.
- App era, framework coverage, signing conditions, execution location, and unsupported capabilities must be documented.
- Some modern/proprietary applications may remain unsupported.
- Remote execution can extend reach but introduces network, cost, privacy, and operations requirements.

## References

- [Apple: Running an app on simulated or physical devices](https://developer.apple.com/documentation/xcode/running-your-app-on-simulated-or-physical-devices)
- [touchHLE](https://github.com/touchHLE/touchHLE) — a high-level emulator project whose own documentation describes a limited target of early iOS apps; not evidence of modern universal compatibility.
