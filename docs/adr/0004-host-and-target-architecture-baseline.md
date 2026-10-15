# ADR 0004: Baseline Host OS and Target CPU Architectures

- **Status:** Accepted
- **Date:** 2026-10-15
- **Decision owners:** Engineering lead, System architect

## Context

Universal Mobile Runtime (UMR) targets desktop users across Windows, macOS, and Linux. The host environments have disparate kernel interfaces, hypervisors, graphics stacks, and CPU architectures (x86-64 and ARM64). Additionally, mobile guest applications are predominantly built for ARM64 and ARMv7, with a subset offering x86-64 native libraries or pure managed bytecode. We must establish a clear host and architecture support tiering to scope initial development and avoid fragmented engineering effort.

## Decision drivers

- Commercial availability and developer prevalence of target host OS versions and hardware.
- Native hypervisor and virtualization API availability without requiring third-party kernel drivers.
- Guest mobile app ABI distribution (over 70% of NDK Android apps and 100% of modern iOS store binaries target ARM64).
- Maintenance overhead of dynamic binary translation (DBT) across cross-architecture targets.

## Options considered

1. **x86-64 only hosts:** Excludes Apple Silicon Macs (the dominant macOS developer hardware) and modern ARM64 Windows laptops.
2. **ARM64 only hosts:** Excludes vast numbers of Windows and Linux x86-64 desktop and laptop installations.
3. **Tiered multi-host baseline (Selected):**
   - **Tier 1 Hosts:**
     - Windows 11 (64-bit, x86-64 and ARM64) with Windows Hypervisor Platform (WHPX).
     - macOS 14+ (Apple Silicon ARM64) with `Virtualization.framework`.
     - Linux (Ubuntu 22.04/24.04 LTS, x86-64 and ARM64) with KVM and Binderfs.
   - **Tier 2 Hosts:**
     - Windows 10 (22H2 x86-64).
     - macOS on Intel x86-64 (sunset maintenance).
     - Other Linux distributions (Fedora, Arch) with glibc 2.35+ and Kernel 6.1+.

## Decision

Adopt the Tier 1 baseline for primary implementation and automated CI testing. For guest app architectures:
- **Pure managed code (Java/Kotlin/Dex):** Fully supported across all host CPU architectures via guest VM ART.
- **Native code (ARM64 on ARM64 host, x86-64 on x86-64 host):** Direct hardware execution via virtualization.
- **Cross-architecture native code (ARM64 native binaries on x86-64 host):** Explicitly gated behind dynamic binary translation (DBT) or native bridge technology; compatibility status must be flagged as "Translated" or "Unsupported" if no translation runtime is active.

## Consequences

### Positive
- Matches developer and QA hardware realities (Apple Silicon for macOS, x86-64 and Snapdragon ARM64 for Windows/Linux).
- Allows zero-translation near-native performance for ARM64 mobile apps on Apple Silicon macOS and ARM64 Linux/Windows.
- Avoids custom kernel driver installations by relying on standard host hypervisors (WHPX, Hypervisor.framework, KVM).

### Negative / trade-offs
- Testing matrix requires multi-architecture runner infrastructure.
- x86-64 hosts running ARM-only native libraries require binary translation or will fail to load native libraries.

### Risks and mitigations
- *Risk:* Lack of open-source, performant, clean-room ARM-to-x86 translation for Android on Windows/Linux.
- *Mitigation:* Explicitly identify pure bytecode vs multi-ABI vs ARM-only apps during package intake; flag translation dependencies upfront in diagnostics.

## Revisit when

- Windows on ARM or Linux ARM64 market share shifts significantly.
- High-performance open-source ARM-to-x86 translation libraries achieve production maturity.

