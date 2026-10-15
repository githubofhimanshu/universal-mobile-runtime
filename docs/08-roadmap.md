# Roadmap

This roadmap is a sequence of evidence-based gates, not a promise of calendar dates. Size work only after feasibility results are available.

## Phase 0 — Feasibility and product decisions

- Choose first host OS versions and CPU architectures.
- Run Android execution spikes on each desktop host.
- Run iOS execution spikes and select an explicitly described support target.
- Review dependencies, licensing, signing, legal basis, and packaging constraints.
- Define a representative, authorized test corpus.

**Exit:** a written evidence pack, backend strategy per host, support boundaries, and approved ADRs.

## Phase 1 — Vertical slice

- Minimal shell and runtime orchestrator.
- One Android launch workflow on a selected host.
- One iOS launch/execution workflow matching the approved feasibility strategy.
- Diagnostics, stop/shutdown, and failure isolation.

**Exit:** a reproducible demo that uses real execution, not placeholders.

## Phase 2 — Android backend depth

- Package lifecycle and runtime management.
- Graphics, input, audio, filesystem, and networking as required by the test corpus.
- Compatibility reporting and performance baseline.

**Exit:** agreed Android test cohort passes on the first host/backend target.

## Phase 3 — Expand across desktop hosts

- Port host adapters and validate per-host Android backend strategies.
- Expand CPU architecture support only where justified by the app corpus.
- Add packaging, install/uninstall, logging, crash recovery, and upgrades.

**Exit:** advertised Android combinations pass the matrix.

## Phase 4 — iOS backend expansion

- Improve the chosen iOS strategy against the supported app cohort.
- Track framework, signing, graphics, and external service gaps.
- Add hosts only where technically and legally supportable.

**Exit:** explicitly supported iOS cohort passes its acceptance tests on every advertised configuration. If the intended target cannot be met, stop and make an explicit scope decision.

## Phase 5 — Distribution and ecosystem

- Curated catalog, publisher onboarding, update signing, provenance, support reports, and licensing workflow.
- Optional remote runtime only with explicit privacy/security terms and clear UI labeling.

**Exit:** distribution/security review and operations plan are complete.

## Later

Android/iOS as host systems, consumer app-store features, cloud execution at scale, richer graphics translation, and broad cross-architecture execution are future opportunities, not Round 1 commitments.
