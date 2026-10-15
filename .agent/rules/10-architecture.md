# Architecture Rules

- Keep UI/presentation code out of binary loading, package parsing, and guest execution.
- Keep the runtime orchestrator independent from concrete Android/iOS backend implementations.
- Each backend must report capabilities, version, execution mode, requirements, and known limitations.
- Keep host-specific OS APIs behind host adapters where practical.
- Use explicit lifecycle states and structured errors; avoid boolean success values that lose diagnostic context.
- Do not confuse guest CPU translation with guest OS ABI/framework compatibility.
- Avoid a custom hypervisor, CPU translator, kernel emulator, or full framework reimplementation unless an ADR documents the need and feasible maintenance plan.
- Consider IPC/process isolation for untrusted native runtime code; the UI process should not load untrusted app native libraries.
- Do not let a test stub/mock be selected as a production backend by default.
- Never represent a remote session as local execution.
