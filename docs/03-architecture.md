# System Architecture

## Architectural approach

Use one product shell and orchestration model over multiple replaceable execution backends. Do not assume a single low-level runtime can behave identically on all host systems.

```text
+--------------------------------------------------------------+
|                      Desktop Application                     |
|  Library | Import | Launch/Stop | Settings | Diagnostics      |
+------------------------------+-------------------------------+
                               |
+------------------------------v-------------------------------+
|                    Runtime Orchestrator                      |
| Capability discovery | Lifecycle | Events | Session policy   |
+----------------------+----------------------+----------------+
                       |                      |
+----------------------v------+   +-----------v----------------+
| Android Backend             |   | iOS Backend                |
| guest/runtime integration   |   | feasibility-selected path  |
+---------------+-------------+   +-------------+--------------+
                |                               |
+---------------v-------------------------------v--------------+
| Shared Host Services                                            |
| OS adapter | Process/session isolation | Storage | Input       |
| Graphics abstraction | Audio | Networking | Permissions       |
+-------------------------------+--------------------------------+
                                |
+-------------------------------v--------------------------------+
|                    Windows / macOS / Linux                      |
+-----------------------------------------------------------------+
```

## Component responsibilities

### Desktop shell

Owns presentation, user actions, app list, settings, status, and diagnostics. It must not parse or execute guest binaries directly.

### Runtime orchestrator

Owns backend discovery, capability negotiation, lifecycle transitions, cancellation, session tracking, event routing, and structured error reporting. It does not emulate a guest OS.

### Host adapter

Encapsulates host-specific filesystem conventions, process management, window integration, input, audio, graphics surfaces, virtualization APIs, and permission prompts. Capabilities must be queried at runtime rather than inferred only from OS names.

### Android backend

Owns Android package handling and the selected Android environment. Depending on the host, it may use a VM, container, emulator, or other licensed integration. ART, JNI, Bionic, Binder, framework services, and vendor components are interdependent; avoid describing Android support as a handful of syscall shims.

### iOS backend

Owns iOS package validation and the chosen execution approach. The strategy must be based on the feasibility research in `docs/research/feasibility-spikes.md`. Local high-level emulation, an Apple-supported simulator workflow, and remote-device execution are materially different approaches and must not be conflated.

### Compatibility/diagnostics service

Maintains the versioned support matrix, runtime capability reports, known incompatibilities, logs, and reproduction bundle generation.

## Interface sketch

Backend contract should eventually expose concepts similar to:

- `backend_id()` and `backend_version()`
- `detect_capabilities(host_context)`
- `inspect_package(package_ref)`
- `prepare_app(app_ref, session_config)`
- `launch(session_id)`
- `stop(session_id)`
- `query_status(session_id)`
- `collect_diagnostics(session_id, consent_policy)`

This is conceptual pseudocode, not a final public API. Define types, ownership, threading, error taxonomy, and asynchronous behavior before implementation.

## Important separation of concerns

1. CPU instruction translation is separate from guest ABI and system services.
2. Package parsing is separate from runtime loading/execution.
3. Graphics API translation is separate from rendering surfaces and window management.
4. Shared UI is separate from backend-specific lifecycle and limitations.
5. Local execution is separate from remote execution.
6. The compatibility matrix is evidence storage, not a marketing list.

## Process model

Prefer process isolation for untrusted app runtimes where feasible. Use explicit IPC, bounded messages, timeouts, cancellation, and crash recovery. Do not load untrusted native code into the main UI process.

## Failure behavior

- Backend unavailable: report required capability/dependency and next diagnostic step.
- Unsupported package: provide a reason without attempting execution.
- Prepare/launch timeout: cancel, stop associated processes, preserve a diagnostic report.
- Backend crash: keep the shell alive, mark the session failed, and avoid automatic repeated relaunch loops.
- Unsupported hardware/service: report the missing capability to the app compatibility record.
