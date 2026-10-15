# Workflow: Implement a Feature

1. Confirm acceptance criteria and follow the approved plan.
2. Inspect current code and conventions; do not assume intended code already exists.
3. Implement one coherent vertical slice and keep changes focused.
4. Add or update tests alongside implementation.
5. Run the relevant formatter, static analysis, build, unit tests, and integration tests available in the environment. Report unavailable tests explicitly.
6. Exercise failure and cleanup paths, especially around processes, files, and runtime sessions.
7. Review changed code for resource ownership, concurrency, error propagation, security, platform guards, and logging of sensitive data.
8. Update docs and support evidence where behavior or claims changed.
9. Report actual outcomes. Never state “all tests pass” unless they were run and passed.

## Required completion report

- Implemented
- Files changed
- Tests run and results
- Host/CPU/backend combinations tested
- Known limitations or untested paths
- Security/license notes
- Follow-up items
