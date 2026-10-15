# Host and Application Support Matrix

Update this file only from reproducible tests or clearly labelled manual results. Do not mark a combination “supported” because it compiles or because the package can be parsed.

## Status meanings

- **Not evaluated:** No feasibility result yet.
- **Research:** Experiments are in progress; no support claim.
- **Prototype:** A demonstration works for a narrow case; not production support.
- **Partial:** A documented subset works, with known limitations.
- **Verified:** Release acceptance tests passed for the stated configuration.
- **Unsupported:** Known not to work for the declared case.
- **Blocked:** Depends on unavailable permissions, services, hardware, or external requirements.

## Platform Matrix (Post-Phase 0 Baseline)

| Host OS | Host CPU | Android Strategy | Android Status | iOS Strategy | iOS Status |
|---|---|---|---|---|---|
| **Windows 11 / 10** | x86-64 | WHPX MicroVM (VirtIO-GPU, VirtIO-vsock) | **Research** (Phase 0 Spikes Complete) | Track B: HLE (Legacy 32-bit); Track C: Tethered Device / Remote Stream | **Research** (Dual-track defined; Local modern unsupported) |
| **Windows 11** | ARM64 | WHPX MicroVM (Native ARM64 AOSP) | **Research** (Native guest execution path defined) | Track B: HLE (Legacy 32-bit); Track C: Tethered Device / Remote Stream | **Research** (Dual-track defined; Local modern unsupported) |
| **macOS 14+** | Apple Silicon (ARM64) | Virtualization.framework MicroVM (Native ARM64 AOSP) | **Research** (Native guest execution path defined) | Track A: Apple Simulator Runtime (Local); Track B: HLE (Legacy 32-bit) | **Research** (Track A/B defined; Simulator path viable) |
| **macOS 14+** | Intel (x86-64) | Virtualization.framework MicroVM (x86-64 AOSP) | **Research** (MicroVM path defined) | Track A: Apple Simulator Runtime (Local); Track B: HLE (Legacy 32-bit) | **Research** (Track A/B defined; Simulator path viable) |
| **Linux (Ubuntu 22.04+)**| x86-64 | Container (namespaces, cgroups v2, Binderfs) | **Research** (Container path defined; near-native) | Track B: HLE (Legacy 32-bit); Track C: Tethered Device / Remote Stream | **Research** (Dual-track defined; Local modern unsupported) |
| **Linux (Ubuntu 24.04+)**| ARM64 | Container (namespaces, cgroups v2, Binderfs) | **Research** (Container path defined; full native) | Track B: HLE (Legacy 32-bit); Track C: Tethered Device / Remote Stream | **Research** (Dual-track defined; Local modern unsupported) |

## Per-app test record

| Test ID | App Name / Package ID | Format | Host OS / CPU | Backend / Version | Execution Mode | Result | Notes / Date |
|---|---|---|---|---|---|---|---|
| `TEST-P1-AND-01` | `org.umr.synthetic.hello` | APK | Windows 11 / x86-64 | `android-reference-backend` 0.1.0 | `LocalNative` | **Verified** | Full lifecycle pass (Prepare -> Run -> Heartbeat -> Stop -> Logs) (2026-10-15) |
| `TEST-P1-AND-NEG` | `malformed-slip.apk` | APK | Windows 11 / x86-64 | `umr-intake-worker` 0.1.0 | N/A | **Verified** | Rejection verified: `ERR_INTAKE_ZIP_TRAVERSAL` (2026-10-15) |
| `TEST-P1-IOS-DRM` | `com.umr.encrypted.mock` | IPA | Windows 11 / x86-64 | `umr-orchestrator` 0.1.0 | N/A | **Verified** | Detected FairPlay cryptid=1; rejected with `ERR_INTAKE_DRM_PROTECTED` (2026-10-15) |
| `TEST-P1-IOS-REM` | `com.umr.sim.mock` | IPA | Windows 11 / x86-64 | `umr-orchestrator` 0.1.0 | `RemoteStream` | **Verified** | Evaluated on non-macOS; classified as `RemoteRequired` / `RemoteStream` (2026-10-15) |
| `TEST-P2-IPC-01` | `umr-ipc-gateway` | JSON-RPC | Windows 11 / x86-64 | `umr-ipc` 0.1.0 | `LocalNative` | **Verified** | Schema-validated async JSON-RPC 2.0 loopback server on port 9090 (2026-10-15) |
| `TEST-P2-INP-01` | `android-input-subsystem` | Events | Windows 11 / x86-64 | `android-reference-backend` 0.1.0 | `LocalNative` | **Verified** | Normalized pointer-to-pixel mapping & navigation keycode dispatch (2026-10-15) |
| `TEST-P2-STG-01` | `android-storage-sandbox` | Sandbox | Windows 11 / x86-64 | `android-reference-backend` 0.1.0 | `LocalNative` | **Verified** | Segmented `/data/data/<pkg>` and `/sdcard/` with verified wipe (2026-10-15) |
| `TEST-P2-FB-01` | `android-framebuffer-pipe` | RGBA8888 | Windows 11 / x86-64 | `android-reference-backend` 0.1.0 | `LocalNative` | **Verified** | 60 FPS RGBA8888 compositor with status bar, touch indicator & nav bar (2026-10-15) |
| `TEST-P2-SHELL-01` | `apps/desktop-shell` | HTML5/TS | Windows 11 / x86-64 | `umr-desktop-shell` 0.1.0 | `LocalNative` | **Verified** | App Library, Dropzone, Phone Canvas Viewport & Diagnostics Drawer (2026-10-15) |

## Required cohorts

Android:

- A simple managed-language app.
- A JNI/native-library app.
- A graphics-intensive app.
- A networking/storage example.
- An app relying on external services, recorded as a separate risk cohort.

 iOS:

- A legally obtained app within the candidate backend's intended era/framework range.
- A basic UI/framework workflow.
- A graphics/input workflow if relevant.
- A test identifying signing, entitlement, service, or device dependence.

The iOS cohort must validate the selected execution approach, not simply the existence of a file loader.
