# ADR 0001: Desktop-First Round 1

- **Status:** Proposed — confirm after reviewing the feasibility gates
- **Date:** 2026-10-15
- **Decision owners:** Project owner and engineering lead

## Context

The long-term product aims to run Android and iOS applications across multiple host operating systems. Starting with mobile hosts, desktop hosts, both app families, a store, and a custom runtime simultaneously creates too many unverified dependencies.

## Decision

Round 1 focuses on Windows, macOS, and Linux as hosts, with Android and iOS execution both explicit workstreams. Android/iOS as host platforms and a public app store are deferred.

## Consequences

- Shared desktop UX and orchestration can be developed while backend implementations remain separate.
- Each advertised host/app combination requires evidence.
- iOS is a release-critical feasibility risk; it cannot be silently deferred or represented by a mock interface.
- Product success is a defined supported app cohort, not a claim of universal compatibility.

## Revisit when

Feasibility evidence shows the required iOS target cannot be supported as stated, or a major legal/licensing/platform limitation requires an explicit scope change.
