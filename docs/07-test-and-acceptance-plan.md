# Test and Acceptance Plan

## Testing layers

1. **Unit tests:** package metadata validation, capability negotiation, lifecycle state machine, path handling, error mapping, compatibility record serialization.
2. **Component tests:** host adapters, IPC, logging, process supervisor, graphics abstraction, storage mapping.
3. **Backend integration tests:** real Android/iOS backend setup, package preparation, launch, interaction, and shutdown.
4. **End-to-end tests:** import in UI, inspect support status, launch, stop, remove, export diagnostics.
5. **Security tests:** malformed archives, path traversal, decompression bombs, invalid metadata, hostile IPC, permission denial, timeout cleanup.
6. **Compatibility tests:** app cohort × host OS × host CPU × backend version × graphics configuration.
7. **Performance tests:** startup time, CPU/GPU use, RAM, frame pacing, input latency, idle use, and crash rate where measurable.

## Minimum end-to-end smoke test

For each advertised combination:

1. Start UMR with a clean user profile.
2. Import an authorized test package.
3. Confirm format and compatibility diagnostics are correct.
4. Prepare and launch the app.
5. Complete a scripted representative workflow.
6. Verify input, rendering, and required services for that workflow.
7. Stop the app and confirm associated processes/resources are released.
8. Relaunch and remove the app; verify expected data-retention behavior.
9. Trigger an unsupported or invalid-package case and confirm the shell stays responsive.
10. Export the diagnostic report and confirm it contains no secrets beyond the documented policy.

## Cross-platform policy

Build and test on native CI runners where practical. A green compile on one host does not establish runtime support on another. For any test that must be manual, record the machine, OS version, CPU, steps, date, result, and tester. Virtualization and GPU behavior may not be fully testable in generic hosted CI runners.

## Release decision

A release claim must point to the relevant compatibility matrix entries. A UI feature can pass while a runtime backend fails; report these separately. Missing iOS evidence cannot be replaced with package acceptance or a fake launch animation.

## Test artifact management

- Do not commit third-party APK/IPAs unless redistribution is explicitly allowed.
- Prefer synthetic or project-owned fixtures.
- Store hashes and instructions for obtaining authorized external fixtures when appropriate.
- Sanitize logs and avoid publishing proprietary app details without authorization.
