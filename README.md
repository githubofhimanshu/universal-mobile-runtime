# Universal Mobile Runtime (working name)

> Documentation starter kit for a desktop-first platform intended to execute Android and iOS applications on Windows, macOS, and Linux.

**Status:** Pre-implementation / feasibility planning  
**Round 1 target:** Windows + macOS + Linux hosts; Android + iOS application backends  
**Code status:** This bundle contains project documentation and agent guidance only. It does not contain an implementation.

## Start here

1. Read [`docs/00-project-brief.md`](docs/00-project-brief.md) for the vision and product framing.
2. Read [`docs/02-round-1-scope.md`](docs/02-round-1-scope.md) for the first release boundary and success criteria.
3. Read [`docs/03-architecture.md`](docs/03-architecture.md) and [`docs/04-runtime-backends.md`](docs/04-runtime-backends.md) before creating runtime abstractions.
4. Read [`docs/research/feasibility-spikes.md`](docs/research/feasibility-spikes.md) before committing to an iOS implementation strategy.
5. Read [`AGENTS.md`](AGENTS.md) and [`.agent/README.md`](.agent/README.md) before vibe coding with an AI agent.

## Documentation map

| File | Purpose |
|---|---|
| `docs/00-project-brief.md` | Vision, problem, users, product principles |
| `docs/01-product-requirements.md` | Product requirements and user journeys |
| `docs/02-round-1-scope.md` | Round 1 scope, non-goals, acceptance criteria |
| `docs/03-architecture.md` | System components and boundaries |
| `docs/04-runtime-backends.md` | Android/iOS execution strategy and constraints |
| `docs/05-platform-support-matrix.md` | Host, architecture, and application support tracking |
| `docs/06-security-privacy.md` | Threat model and security requirements |
| `docs/07-test-and-acceptance-plan.md` | Test approach and release gates |
| `docs/08-roadmap.md` | Phased delivery plan |
| `docs/09-repository-layout.md` | Suggested codebase layout |
| `docs/research/feasibility-spikes.md` | Time-boxed experiments to de-risk design |
| `docs/adr/` | Architecture Decision Records (ADRs) |
| `.agent/` | Tool-neutral context, rules, workflows, and templates for coding agents |
| `AGENTS.md` | Short repository-wide instructions for agents that support this convention |

## Important product boundary

The ambition is to support Android and iOS applications on all three desktop hosts. Do not claim broad or universal iOS compatibility until a tested execution path exists. CPU instruction translation, package loading, and graphics translation alone do not reproduce the Android or iOS frameworks and OS services. A remote iOS device can be an option for evaluation, but it must be labelled remote execution and not presented as local execution.

The first engineering milestone is a **feasibility evidence pack**, not a polished launcher: demonstrate what can run, on which host/CPU, by which execution mechanism, with what restrictions, and under which legal/licensing assumptions.

## Working conventions

- Treat requirements marked **MUST** as release requirements unless an approved ADR changes them.
- Keep assumptions explicit. Convert critical assumptions into experiments or ADRs.
- Keep platform-specific execution code behind backend interfaces.
- Record app compatibility as measured evidence, not marketing claims.
- Do not copy, bundle, or redistribute third-party app binaries, OS images, or proprietary runtime components without confirming rights and license terms.

## Primary references

- [Apple: Running an app on simulated or physical devices](https://developer.apple.com/documentation/xcode/running-your-app-on-simulated-or-physical-devices)
- [QEMU: Translator Internals / TCG](https://www.qemu.org/docs/master/devel/tcg.html)
- [Waydroid documentation](https://github.com/waydroid/docs)
- [touchHLE: high-level emulator for early iOS apps](https://github.com/touchHLE/touchHLE)
- [MoltenVK](https://github.com/KhronosGroup/MoltenVK)

These projects are references for investigation, not automatically approved dependencies or evidence that all target apps/platforms are supported.
