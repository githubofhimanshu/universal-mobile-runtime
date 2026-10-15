# ADR 0005: Android Backend Architecture Across Desktop Hosts

- **Status:** Accepted
- **Date:** 2026-10-15
- **Decision owners:** Android Backend Lead, System Architect

## Context

Android runtime execution requires the Android Open Source Project (AOSP) environment: Android Runtime (ART), Bionic libc, Binder IPC, `system_server`, Hardware Abstraction Layer (HAL), and graphics driver pipelines. This cannot be solved by simple user-space syscall shims. Furthermore, the three target desktop operating systems provide different isolation primitives and kernel capabilities:
- Linux supports kernel namespaces, cgroups v2, and in-kernel Binder (`/dev/binderfs`).
- Windows and macOS run different kernels (NT and XNU) and cannot natively execute Linux/Android containers without a Linux kernel virtualization boundary.

## Decision drivers

- Performance, boot latency, and resource footprint.
- Security isolation between untrusted Android applications and the host OS.
- Minimizing proprietary licensing barriers and custom kernel driver requirements.
- Unified orchestration interface despite heterogeneous per-host execution mechanics.

## Options considered

1. **Monolithic Emulator across all hosts (e.g. customized QEMU full system emulation):**
   - *Pros:* Uniform codebase across all platforms.
   - *Cons:* Heavy resource consumption, slow boot times, redundant overhead on Linux where native containerization is possible.
2. **Reimplementing Android Framework in user-space (libndk/Wine-like approach for Android):**
   - *Pros:* Light memory footprint.
   - *Cons:* Immense multi-year maintenance burden; failure to keep up with Android API levels; Binder/SurfaceFlinger complexity.
3. **Host-Tailored Execution Strategy behind Unified Backend Contract (Selected):**
   - **Linux:** Container-based execution using LXC/namespaces + cgroups + Binderfs (following Waydroid architectural model) with direct Wayland/EGL rendering.
   - **Windows:** MicroVM running an AOSP guest kernel via Windows Hypervisor Platform (WHPX) with VirtIO-GPU/VirtIO-vsock for display and IPC.
   - **macOS:** MicroVM via Apple `Virtualization.framework` running an AOSP guest with VirtIO devices and Metal-backed rendering.

## Decision

Adopt Option 3. Encapsulate execution inside platform-specific runtime engines that all implement the unified `IAndroidBackend` lifecycle and orchestration contract. The Desktop Shell and Orchestrator interact solely with `IAndroidBackend`.

## Interface and Lifecycle Contract

The Android backend contract will provide:
- `backend_id()`: String identifier (e.g. `android-linux-container`, `android-win-whpx`, `android-mac-virt`).
- `capabilities()`: Supported ABI list, max API level, GLES/Vulkan capabilities, hardware acceleration status.
- `intake_package(path)`: Safe uncompressed inspection, manifest analysis, ABI check.
- `prepare(session_config)`: Allocate container/microVM resources, mount package app sandbox.
- `start(session_id)`: Launch target activity, establish display stream/surface and input channel.
- `stop(session_id)`: Graceful signal dispatch, timeout enforcement, process kill, resource release.
- `get_diagnostics(session_id)`: Retrieve logcat filtered to application UID without leaking system secrets.

## Consequences

### Positive
- Maximum performance on Linux (zero virtualization penalty; direct GPU access).
- Clean, robust security boundary on Windows and macOS via hardware hypervisor isolation.
- Extensible backend interface allows swapping the underlying VM or container manager without touching the UI shell.

### Negative / trade-offs
- Requires maintaining two distinct guest runtime pipelines (Linux container runner vs. Windows/macOS MicroVM supervisor).
- Packaging and distributing lightweight AOSP guest rootfs/kernel images for Windows and macOS.

### Risks and mitigations
- *Risk:* Distribution size of AOSP guest system image for Windows/macOS.
- *Mitigation:* Use minimal headless AOSP images (e.g., Cuttlefish/micro-AOSP derived) stripped of unused system apps and vendor blobs; enable on-demand downloading with integrity verification.

## Revisit when

- Microsoft WSL2 or Apple Virtualization framework introduces standardized container interfaces that eliminate the need for custom microVM management.

