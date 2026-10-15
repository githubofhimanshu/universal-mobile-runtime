# Security, Privacy, and Distribution Requirements

This document is engineering guidance, not legal advice. Obtain qualified review before redistributing apps, OS images, runtime components, or signing-related material.

## Threat model

UMR processes user-selected packages and may execute third-party native code. Assume package contents, filenames, metadata, app resources, runtime messages, and app logs can be malicious or malformed.

Threats include:

- Memory corruption or parser vulnerabilities in package/metadata processing.
- Malicious native code escaping the runtime boundary.
- Excessive filesystem, microphone, camera, clipboard, network, or device access.
- Supply-chain compromise through runtime images or third-party dependencies.
- Signing/entitlement misuse or accidental weakening of platform security.
- Sensitive information leaking through diagnostic logs, crash dumps, or remote sessions.
- Redistribution of copyrighted apps, OS components, or incompatible licenses.

## Required controls

1. Treat archives and packages as untrusted input; enforce size, path, recursion, and decompression limits.
2. Validate canonical paths and prevent path traversal, symlink abuse, and writes outside managed directories.
3. Keep parsers and runtime processes isolated from the UI where feasible.
4. Apply least privilege. Request host capabilities only when necessary and with user-visible consent.
5. Bound IPC message sizes, request rates, timeouts, and resource consumption.
6. Use authenticated and integrity-checked update channels for UMR components.
7. Keep secrets, private keys, signing certificates, access tokens, and proprietary runtime images out of source control and logs.
8. Redact sensitive data by default; require user consent before collecting/exporting diagnostics.
9. For remote execution, secure transport, authenticate sessions, isolate user data, explain data residency/retention, and provide session termination controls.
10. Record third-party dependencies, licenses, notices, and redistribution rights before bundling them.
11. Do not implement a bypass of platform signing, DRM, app integrity, or access controls as a workaround.
12. Maintain a vulnerability reporting process before external release.

## Data handling defaults

- App packages stay local unless a user explicitly chooses a remote workflow.
- Logs should not collect app content, credentials, keystrokes, or network payloads by default.
- Crash uploads are opt-in and show what will be sent.
- App removal explains whether user data is retained or deleted.
- Test apps must have documented provenance and test/redistribution permission.

## Release security gates

- Dependency and license inventory reviewed.
- Package parsing tests include malformed and adversarial inputs.
- Runtime isolation and permission boundaries documented.
- Secret scanning and dependency scanning enabled in CI when code is present.
- No hard-coded secrets, downloaded binary surprises, or unreviewed update mechanism.
- Threat model revisited when remote execution, app-store functionality, or new device services are introduced.
