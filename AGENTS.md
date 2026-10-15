# Repository Instructions for AI Coding Agents

Read these instructions before planning or changing code. The more detailed, tool-neutral agent guidance lives in [`.agent/`](.agent/README.md).

## Product context

This project aims to build a desktop-first mobile application runtime for **Windows, macOS, and Linux hosts**, with separate execution backends for Android and iOS applications. Round 1 includes both app families as product goals, but broad iOS compatibility is an explicit feasibility risk. Do not quietly replace that goal with a mock UI or assume that accepting an `.ipa` means the app can run.

## Required behavior

1. Before implementing a non-trivial feature, read the related product requirement, architecture document, and ADRs.
2. For tasks that affect runtime compatibility, identify the host OS, host CPU, guest/app CPU, execution method, and test evidence.
3. Prefer a small, testable vertical slice over broad scaffolding.
4. Do not introduce a custom CPU translator, kernel emulator, framework reimplementation, or package manager unless the plan demonstrates why an existing licensed component or simpler approach is inadequate.
5. Keep Android and iOS backend concerns separate behind stable interfaces. Shared UI must not become a repository for platform-specific execution logic.
6. Treat all imported app packages, metadata, filenames, manifests, and runtime output as untrusted input.
7. Do not disable signing checks, sandboxing, permissions, tests, or security controls merely to make a demo work. Any test-only bypass must be isolated, documented, and impossible to ship accidentally.
8. Do not add binary blobs, proprietary OS images, app packages, keys, certificates, or credentials to the repository.
9. Do not claim a feature works on an OS/architecture combination without a reproducible test or clearly marked manual verification.
10. For architectural uncertainty, propose an experiment and document the result. Update or add an ADR for significant decisions.

## Completion checklist

- [ ] Requirements and unsupported cases are documented.
- [ ] Tests cover the changed behavior and error paths.
- [ ] Relevant host/architecture combinations are stated.
- [ ] Security and licensing implications are considered.
- [ ] User-facing claims match demonstrated functionality.
- [ ] Documentation and compatibility records are updated.

## Agent files

Use `.agent/rules/` for standing rules, `.agent/workflows/` for repeatable task sequences, and `.agent/templates/` for structured artifacts. Provider-specific files (for example, rules or instruction files for a particular IDE) may be added later by referencing these canonical documents instead of duplicating conflicting rules.
