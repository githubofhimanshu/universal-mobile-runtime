# ADR 0002: Use Multiple Execution Backends

- **Status:** Proposed
- **Date:** 2026-10-15
- **Decision owners:** Engineering lead

## Context

Android and iOS have different executable formats, runtime systems, framework APIs, service models, signing requirements, and graphics dependencies. Windows, macOS, and Linux also expose different virtualization, process, window, and graphics capabilities.

## Decision

Build a common runtime orchestration contract with distinct Android and iOS backends and host-specific adapters. Backend implementations must advertise capabilities and unsupported cases. The UI must not assume the same execution mechanism across all combinations.

## Consequences

- Backend work can be tested and released independently.
- Shared abstractions need careful design and may not cover every platform-specific feature.
- Additional process boundaries and interface design are required.
- The team must avoid an overly generic abstraction before the first real vertical slice proves the required contract.

## Alternatives considered

- One monolithic emulator for both app families: rejected because the runtime models and constraints differ materially.
- Reimplement every OS layer from scratch: rejected as the default due to scope, maintenance, compatibility, and security burden.
