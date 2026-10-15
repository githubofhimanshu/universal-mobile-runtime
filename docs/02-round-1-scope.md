# Round 1 Scope: Desktop Hosts, Android + iOS Apps

## Product commitment

Round 1 targets a desktop product for **Windows, macOS, and Linux** with both **Android and iOS application support** in its planned architecture and validation programme.

The scope is a supported and demonstrable set of applications, not a claim that every app works. Each host/backend combination must be verified independently. “Round 1 includes iOS” means iOS execution is an explicit workstream and release gate, not merely a future placeholder.

## In scope

- Desktop shell and unified app library.
- Host detection, host capability report, logging, crash containment, and diagnostics.
- Package import and safe validation for supported formats.
- Android backend proof of execution, package lifecycle, graphics/input, storage, networking, and app-window integration where feasible.
- iOS backend feasibility work and a demonstrated execution path for a defined, documented app class.
- Backend capability contracts and a host/application compatibility matrix.
- Reproducible tests on the three target desktop operating systems.
- Local and remote execution modes may be evaluated, but must be presented as different product modes.

## Explicitly out of scope for Round 1

- Android or iOS as host operating systems.
- A public app store with third-party publishing and payments.
- Universal execution guarantees for all APKs/IPAs.
- Complete recreation of XNU, the iOS framework stack, Android system services, DRM, hardware attestation, Play Integrity, or Apple-specific cloud services.
- Proprietary app binaries, OS images, signing keys, or credentials committed to source control.
- A new general-purpose CPU translator before a concrete need is demonstrated.

## Release gates

### Gate 0 — feasibility approved

- Host/CPU targets are chosen for initial tests.
- Android strategy per desktop OS has been tested in a spike.
- iOS strategy per desktop OS has a technical evidence report, legal/licensing review, and a clear statement of local versus remote execution.
- Dependencies and redistribution terms have been reviewed.

### Gate 1 — vertical slice

- A real supported Android app can be imported/launched on at least one target host and the failure path is tested.
- A real supported iOS app can be executed using the chosen, explicitly labelled method on at least one target host.
- The shell remains responsive if the app/runtime fails.

### Gate 2 — desktop matrix

- Required smoke-test cases pass on all three desktop operating systems for each advertised feature.
- Unsupported combinations are visibly marked; there are no implied claims of coverage.

### Gate 3 — Round 1 release decision

- Android and iOS results are separately reviewed against the compatibility corpus.
- Every advertised combination has recorded evidence.
- If iOS execution cannot meet the agreed target, product leadership must explicitly revise scope or hold the release. Do not mark the gate passed by substituting a nonfunctional interface.

## Definition of done for a claimed application

A specific app is “verified” only when a repeatable test confirms installation/preparation, launch, a representative interactive workflow, graphics and input behavior as applicable, clean shutdown, and expected logging on the declared host/backend configuration. A package accepted by the file picker is not a verified app.

## Assumptions that must be confirmed

- Supported host CPU architectures (for example, x86-64 and/or ARM64).
- Whether iOS must execute locally or whether remote execution is acceptable for any subset.
- Which representative apps can legally be used as test fixtures.
- Minimum supported OS versions and baseline hardware specifications.
- Whether Google/Apple services, camera, sensors, notifications, or DRM-protected content are launch requirements.
