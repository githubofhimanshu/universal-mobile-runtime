# Feasibility Spikes

Every spike should be time-boxed by the project owner, answer one question, produce reproducible evidence, and end with a decision. The objective is not to prove the whole product in one experiment.

## Spike A — Android on three desktops

**Question:** What is the simplest viable execution path for Android apps on each target host/CPU?

Evaluate guest VM/emulator and host integration options separately for Windows, macOS (Intel/Apple Silicon as applicable), and Linux. Consider virtualization support, guest architecture, GPU acceleration, packaging/distribution, app lifecycle, API level, and license terms.

**Deliverables:** setup scripts or steps, app cohort results, host resource use, launch/stop reliability, graphics/input limitations, dependency and license notes, and proposed backend per host.

## Spike B — iOS application execution

**Question:** What kind of iOS apps can actually execute, by what method, on each target desktop OS?

Compare and separately label:

- Local high-level emulation with finite framework coverage.
- Apple-supported simulator/developer workflows on supported Apple hosts.
- Authorized remote-device execution/streaming.
- Any other legally supportable runtime under investigation.

For each option, record app era/OS, CPU architecture, framework requirements, graphics, signing/entitlements, services, execution location, host availability, legal/licensing assumptions, privacy model, and unsupported cases.

**Required output:** a decision table and a recommendation. A Mach-O parser, Objective-C runtime stub, package picker, or simulated launch screen does not satisfy this spike.

## Spike C — CPU architecture strategy

**Question:** Which initial host and app architectures must be supported, and where is translation required?

Measure the real app cohort first. Compare native execution, hardware virtualization, and existing dynamic binary translation options. Record licensing, performance, debugging, signal/exception behavior, system ABI assumptions, and limitations.

**Rule:** Do not build a custom general-purpose translator unless the evidence shows that it is required and the team has the capacity to own it.

## Spike D — Graphics and window integration

**Question:** What graphics backend and host integration work is needed for the initial app cohort?

Test a simple 2D app and one more demanding graphics workload on each target. Record surface creation, shader support, synchronization, resize/window handling, frame pacing, input latency, GPU availability, and fallback behavior.

Remember: MoltenVK implements Vulkan over Metal; it does not by itself translate arbitrary Metal apps to Vulkan.

## Spike E — Package intake and isolation

**Question:** Can package inspection and runtime launch happen without giving untrusted input control of the shell?

Test malformed packages, oversized archives, path traversal, invalid metadata, interrupted operations, sandbox boundaries, permission denial, process cleanup, and diagnostic redaction.

## Evidence template

- Spike ID / owner / date:
- Question and time box:
- Hosts and CPU models:
- Candidate component/version/license:
- Test application provenance (no binary committed unless permitted):
- Reproduction steps:
- Expected vs actual results:
- Performance/stability observations:
- Unsupported cases and risks:
- Decision: proceed / revise / stop:
- ADR or requirement updated:
