# Workflow: Debug an Issue

1. Reproduce the issue or say clearly why it cannot be reproduced.
2. Capture host OS/version, CPU, backend/version, app fixture/hash, execution mode, and relevant configuration.
3. Reduce to the smallest failing test case without deleting meaningful failure evidence.
4. Check logs, lifecycle events, error codes, process state, permissions, dependencies, and resource cleanup.
5. Form a falsifiable hypothesis and use a targeted experiment; change one important variable at a time.
6. Fix the underlying cause rather than hiding the error, suppressing diagnostics, or adding a broad retry.
7. Add a regression test, including cleanup/timeout behavior if relevant.
8. Run adjacent tests and document untested configurations.
9. Update the compatibility matrix if the issue changes a support claim.

## Report

- Reproduction conditions
- Expected vs actual behavior
- Root cause and evidence
- Fix
- Regression test
- Tests run
- Remaining risks / affected configurations
