# Runtime Backend Strategy

## Principle

Each app family has its own runtime model. “Universal” should describe the user experience and extensible backend interface—not a claim that one emulator can execute all operating systems or binaries.

## Android backend

### Candidate implementation paths

Investigate per host rather than selecting one approach for all desktop systems:

- Linux: container-based Android environment or a virtual machine, subject to kernel, graphics, packaging, and distribution constraints. Waydroid is a useful container-based reference for Linux, but its architecture and licensing must be assessed before integration.
- Windows: an Android guest/emulator/VM strategy compatible with supported Windows versions and CPU virtualization capabilities.
- macOS: an Android VM/emulator strategy that accounts for Intel versus Apple Silicon and the guest ABI.

These may be different implementations behind one `AndroidBackend` contract.

### Compatibility considerations

- APK splits and app bundles versus standalone APKs.
- ARM and x86 native libraries; 32-bit versus 64-bit requirements.
- ART/runtime and Android API level.
- JNI, Bionic, Binder, native services, and framework APIs.
- Graphics, input, audio, storage, networking, clipboard, camera, and sensors.
- Google Play Services, login, integrity checks, DRM, anti-cheat, and vendor-specific behavior.
- App permissions and user-data isolation.

Do not implement binary translation until the test corpus demonstrates a workload/architecture gap and benchmarks justify the cost. Evaluate mature translation/virtualization components and license terms first.

## iOS backend

### Treat as a feasibility workstream

A Mach-O parser/loader alone does not reproduce iOS. Useful support may depend on Objective-C runtime behavior, Foundation/UIKit and other frameworks, system services, graphics, ABI details, signing/entitlements, app version, and device-specific assumptions.

Candidate paths to investigate:

1. **High-level local emulation of a defined app subset.** Potentially useful for selected older apps; framework coverage is limited and must be measured.
2. **Apple-supported simulator/developer workflow on macOS.** Useful for appropriate build/test workflows, but it is not a general solution for arbitrary consumer IPA files on Windows or Linux.
3. **Authorized remote-device execution or streaming.** May demonstrate app use from other desktops, but requires a compatible remote device/service, network, legal authorization, privacy protections, and honest UI labeling. It is not local execution.
4. **A future compatible runtime implementation.** Research only until supported framework coverage and feasibility are demonstrated.

Do not promise arbitrary modern iOS apps on all three desktops at project outset. Define an app cohort, execution location, signing assumptions, supported OS/framework versions, and legal basis for each experiment.

## CPU execution and translation

Prefer native execution when the application and host ABI are compatible, hardware virtualization when appropriate, and dynamic binary translation when required. A translator only addresses instruction-set differences; it does not supply operating-system APIs, application frameworks, drivers, or proprietary services.

Reference for investigation: [QEMU TCG translator internals](https://www.qemu.org/docs/master/devel/tcg.html).

## Graphics

Build a graphics capability abstraction before assuming all APIs can be mapped to Vulkan. Vulkan, Metal, OpenGL ES, and platform window/surface APIs have different capabilities and synchronization semantics. MoltenVK implements Vulkan over Metal; it is not itself a general-purpose Metal-to-Vulkan translator for arbitrary applications.

Reference: [MoltenVK project](https://github.com/KhronosGroup/MoltenVK).

## Decision rule

No backend strategy is accepted based only on a demo video, project description, or package parser. The evidence must include a reproducible build/setup process, a real application, a tested workflow, known failures, measured resource use, legal/license review, and an explicit list of unsupported features.
